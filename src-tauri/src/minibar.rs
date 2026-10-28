//! Windows-only, event-driven GDI mini bar. No WebView or renderer process.
#[cfg(windows)]
mod platform {
    use crate::{model::{now, MiniTone, Snapshot}, runtime};
    use std::{ptr::null_mut, sync::{Arc, Mutex, atomic::{AtomicBool, AtomicIsize, Ordering}}};
    use windows_sys::Win32::{Foundation::*, Graphics::Gdi::*, System::LibraryLoader::GetModuleHandleW, UI::{HiDpi::{GetDpiForWindow, SetThreadDpiAwarenessContext, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2}, WindowsAndMessaging::*}};

    const UPDATE: u32 = WM_APP + 1;
    const BACKGROUND: u32 = 0x00212420;
    fn wide(s: &str) -> Vec<u16> { s.encode_utf16().chain(Some(0)).collect() }
    struct Data { app: tauri::AppHandle, snapshot: Mutex<Option<Snapshot>>, hwnd: AtomicIsize, moving: AtomicBool }
    pub struct MiniBar { data: Arc<Data> }

    impl MiniBar {
        pub fn new(app: tauri::AppHandle) -> Self {
            let data = Arc::new(Data { app, snapshot: Mutex::new(None), hwnd: AtomicIsize::new(0), moving: AtomicBool::new(false) });
            let thread_data = data.clone();
            std::thread::spawn(move || unsafe {
                // This window owns its pixel rendering; do not let Windows stretch a low-DPI bitmap.
                SetThreadDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
                let class = wide("AiTokenNativeMiniBar");
                let instance = GetModuleHandleW(std::ptr::null());
                let wc = WNDCLASSW { lpfnWndProc: Some(proc), hInstance: instance, lpszClassName: class.as_ptr(),
                    hCursor: LoadCursorW(null_mut(), IDC_ARROW), ..std::mem::zeroed() };
                if RegisterClassW(&wc) == 0 { return; }
                let hwnd = CreateWindowExW(WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
                    class.as_ptr(), wide("AI Token 미니바").as_ptr(), WS_POPUP,
                    0, 0, 140, 36, null_mut(), null_mut(), instance, Arc::as_ptr(&thread_data) as *const _);
                if hwnd.is_null() { return; }
                thread_data.hwnd.store(hwnd as isize, Ordering::Release);
                PostMessageW(hwnd, UPDATE, 0, 0);
                // Sleep in GetMessage; arrange starts the timer only while visible.
                let mut msg: MSG = std::mem::zeroed();
                while GetMessageW(&mut msg, null_mut(), 0, 0) > 0 { TranslateMessage(&msg); DispatchMessageW(&msg); }
                thread_data.hwnd.store(0, Ordering::Release);
            });
            Self { data }
        }
        pub fn update(&self, snapshot: &Snapshot) {
            *self.data.snapshot.lock().unwrap() = Some(snapshot.clone());
            let hwnd = self.data.hwnd.load(Ordering::Acquire) as HWND;
            if !hwnd.is_null() { unsafe { PostMessageW(hwnd, UPDATE, 0, 0); } }
        }
    }

    unsafe fn geometry(hwnd: HWND, s: &Snapshot) -> (i32, i32, i32, i32, bool) {
        let scale = GetDpiForWindow(hwnd).max(96) as f64 / 96.0;
        let px = |v: i32| (v as f64 * scale).round() as i32;
        let displayed = s.visible_mini_providers();
        let width = px(16 + 132 * displayed.len() as i32);
        let height = px(40);
        let tray = FindWindowW(wide("Shell_TrayWnd").as_ptr(), std::ptr::null());
        let mut rect: RECT = std::mem::zeroed();
        let found = !tray.is_null() && GetWindowRect(tray, &mut rect) != 0;
        if !found { rect = RECT { left: 0, top: GetSystemMetrics(SM_CYSCREEN)-px(48), right: GetSystemMetrics(SM_CXSCREEN), bottom: GetSystemMetrics(SM_CYSCREEN) }; }
        let notify = FindWindowExW(tray, null_mut(), wide("TrayNotifyWnd").as_ptr(), std::ptr::null());
        let mut notify_rect: RECT = std::mem::zeroed();
        let right = if !notify.is_null() && GetWindowRect(notify, &mut notify_rect) != 0 { notify_rect.left - px(8) } else { rect.right - px(340) };
        let default_x = (right-width).max(rect.left);
        let default_y = rect.top + ((rect.bottom-rect.top-height)/2).max(0);
        let mut x = s.settings.mini_x.unwrap_or(default_x);
        let mut y = s.settings.mini_y.unwrap_or(default_y);
        // Clamp saved positions to a connected monitor after unplugging a display.
        let monitor = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
        let mut info: MONITORINFO = std::mem::zeroed(); info.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        GetMonitorInfoW(monitor, &mut info);
        x = x.clamp(info.rcMonitor.left, (info.rcMonitor.right-width).max(info.rcMonitor.left));
        y = y.clamp(info.rcMonitor.top, (info.rcMonitor.bottom-height).max(info.rcMonitor.top));
        // Visibility depends only on explicit settings and optional process detection,
        // never on focus/fullscreen geometry (which caused flickering previously).
        let visible = !displayed.is_empty();
        (x, y, width, height, visible)
    }

    unsafe fn arrange(hwnd: HWND, s: &Snapshot) {
        let (x,y,w,h,visible) = geometry(hwnd,s);
        let mut current: RECT = std::mem::zeroed(); GetWindowRect(hwnd, &mut current);
        if visible {
            if IsWindowVisible(hwnd) == 0 { SetTimer(hwnd, 1, 2000, None); }
            if current.left != x || current.top != y || current.right-current.left != w || current.bottom-current.top != h || IsWindowVisible(hwnd) == 0 {
                SetWindowPos(hwnd, HWND_TOPMOST, x,y,w,h,SWP_NOACTIVATE | SWP_SHOWWINDOW);
            } else {
                // Explorer can raise its taskbar above a separate overlay. Restore
                // stacking without hiding, resizing, repainting, or taking focus.
                SetWindowPos(hwnd, HWND_TOPMOST,0,0,0,0,SWP_NOACTIVATE|SWP_NOMOVE|SWP_NOSIZE);
            }
        } else {
            KillTimer(hwnd, 1);
            if IsWindowVisible(hwnd) != 0 { ShowWindow(hwnd, SW_HIDE); }
        }
    }

    unsafe fn logo(dc: HDC, id: &str, x: i32, y: i32, size: i32) {
        let img = runtime::service_icon(id, true);
        let mut pixels = Vec::with_capacity(img.rgba().len());
        for p in img.rgba().chunks_exact(4) {
            let a = p[3] as u32;
            // Composite transparency onto this window background without altering the logo.
            pixels.extend_from_slice(&[((p[2] as u32*a+33*(255-a))/255) as u8,
                ((p[1] as u32*a+36*(255-a))/255) as u8, ((p[0] as u32*a+32*(255-a))/255) as u8, 255]);
        }
        let bmi = BITMAPINFO { bmiHeader: BITMAPINFOHEADER { biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: img.width() as i32, biHeight: -(img.height() as i32), biPlanes: 1, biBitCount: 32, biCompression: BI_RGB, ..std::mem::zeroed() }, ..std::mem::zeroed() };
        let old_mode = SetStretchBltMode(dc,HALFTONE);
        let mut old_origin: POINT = std::mem::zeroed();
        SetBrushOrgEx(dc,0,0,&mut old_origin);
        StretchDIBits(dc,x,y,size,size,0,0,img.width() as i32,img.height() as i32,pixels.as_ptr() as *const _, &bmi,DIB_RGB_COLORS,SRCCOPY);
        SetStretchBltMode(dc,old_mode);
        SetBrushOrgEx(dc,old_origin.x,old_origin.y,null_mut());
    }

    fn badge_rect(s: &Snapshot, scale: f64) -> Option<RECT> {
        let count = s.visible_mini_providers().len();
        if count == 0 || !s.updates.has_update() { return None; }
        let px = |v: i32| (v as f64 * scale).round() as i32;
        let right = 16 + 132 * count as i32;
        Some(RECT { left: px(right-24), top: px(8), right: px(right-2), bottom: px(32) })
    }

    fn badge_hit(s: &Snapshot, scale: f64, x: i32, y: i32) -> bool {
        badge_rect(s, scale).is_some_and(|r| x >= r.left && x < r.right && y >= r.top && y < r.bottom)
    }

    unsafe fn draw_badge(dc: HDC, rect: &RECT, scale: f64) {
        let px = |v: i32| (v as f64 * scale).round() as i32;
        let x = rect.left + px(4);
        let y = rect.top + px(4);
        let pen = CreatePen(PS_SOLID, px(1).max(1), 0x00dce6dc);
        let previous_pen = SelectObject(dc, pen);
        let previous_brush = SelectObject(dc, GetStockObject(NULL_BRUSH));
        MoveToEx(dc, x+px(1), y+px(12), null_mut());
        LineTo(dc, x+px(13), y+px(12));
        LineTo(dc, x+px(11), y+px(9));
        LineTo(dc, x+px(11), y+px(5));
        let dome = [POINT { x:x+px(11),y:y-px(1) }, POINT { x:x+px(3),y:y-px(1) }, POINT { x:x+px(3),y:y+px(5) }];
        PolyBezierTo(dc, dome.as_ptr(), dome.len() as u32);
        LineTo(dc, x+px(3), y+px(9));
        LineTo(dc, x+px(1), y+px(12));
        Arc(dc, x+px(5), y+px(11), x+px(9), y+px(16), x+px(5), y+px(13), x+px(9), y+px(13));
        let dot = CreateSolidBrush(0x005d63e5);
        SelectObject(dc, GetStockObject(NULL_PEN));
        SelectObject(dc, dot);
        Ellipse(dc, rect.right-px(6), rect.top+px(1), rect.right-px(1), rect.top+px(6));
        SelectObject(dc, previous_brush);
        SelectObject(dc, previous_pen);
        DeleteObject(dot);
        DeleteObject(pen);
    }

    unsafe fn text_font(scale: f64) -> HFONT {
        let height = (13.0*scale).round() as i32;
        CreateFontW(-height,0,0,0,500,0,0,0,DEFAULT_CHARSET as u32,OUT_DEFAULT_PRECIS as u32,
            CLIP_DEFAULT_PRECIS as u32,ANTIALIASED_QUALITY as u32,DEFAULT_PITCH as u32,wide("Malgun Gothic").as_ptr())
    }

    unsafe fn draw_contents(dc: HDC, area: &RECT, scale: f64, s: &Snapshot) {
        let px = |v: i32| (v as f64*scale).round() as i32;
        let bg = CreateSolidBrush(BACKGROUND); FillRect(dc,area,bg); DeleteObject(bg);
        SetBkMode(dc,TRANSPARENT as i32);
        let font = text_font(scale);
        let old = SelectObject(dc,font);
        SetTextColor(dc,0x00889988);
        let mut grip = RECT { left:px(2),top:0,right:px(14),bottom:area.bottom };
        DrawTextW(dc,wide(if s.settings.mini_locked { "·" } else { "⋮" }).as_ptr(),-1,&mut grip,DT_CENTER|DT_VCENTER|DT_SINGLELINE);
        let displayed = s.visible_mini_providers();
        let badge = badge_rect(s, scale);
        for (index,&id) in displayed.iter().enumerate() {
            let left = px(16+index as i32*132);
            logo(dc,id,left,px(8),px(24));
            let lines = crate::model::mini_lines(s.providers.iter().find(|p| p.id==id), now());
            for (row,(text,tone)) in lines.iter().enumerate() {
                SetTextColor(dc,match tone {
                    MiniTone::Unknown => 0x00a4b1a4,
                    MiniTone::Normal => 0x00ecf0ed,
                    MiniTone::Low => 0x008ec0ff,
                    MiniTone::Critical => 0x009b9bff,
                });
                let text_right = if badge.is_some() && index+1 == displayed.len() { 104 } else { 128 };
                let mut r = RECT { left:left+px(32),top:px(2+row as i32*18),right:left+px(text_right),bottom:px(20+row as i32*18) };
                DrawTextW(dc,wide(text).as_ptr(),-1,&mut r,DT_LEFT|DT_VCENTER|DT_SINGLELINE|DT_END_ELLIPSIS);
            }
        }
        if let Some(rect) = badge { draw_badge(dc, &rect, scale); }
        SelectObject(dc,old); DeleteObject(font);
    }

    unsafe fn paint(hwnd: HWND, s: &Snapshot) {
        let mut ps: PAINTSTRUCT = std::mem::zeroed(); let window_dc = BeginPaint(hwnd,&mut ps);
        let scale = GetDpiForWindow(hwnd).max(96) as f64/96.0;
        let mut area: RECT = std::mem::zeroed(); GetClientRect(hwnd,&mut area);
        let buffer = CreateCompatibleDC(window_dc);
        let bitmap = CreateCompatibleBitmap(window_dc,(area.right-area.left).max(1),(area.bottom-area.top).max(1));
        let buffered = !buffer.is_null() && !bitmap.is_null();
        let dc = if buffered { buffer } else { window_dc };
        let old_bitmap = if buffered { SelectObject(dc,bitmap) } else { null_mut() };
        draw_contents(dc, &area, scale, s);
        if buffered {
            BitBlt(window_dc,0,0,area.right-area.left,area.bottom-area.top,dc,0,0,SRCCOPY);
            SelectObject(dc,old_bitmap);
        }
        if !bitmap.is_null() { DeleteObject(bitmap); }
        if !buffer.is_null() { DeleteDC(buffer); }
        EndPaint(hwnd,&ps);
    }

    unsafe extern "system" fn proc(hwnd: HWND,msg: u32,w: WPARAM,l: LPARAM) -> LRESULT {
        if msg == WM_NCCREATE {
            let cs = &*(l as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd,GWLP_USERDATA,cs.lpCreateParams as isize);
        }
        let ptr = GetWindowLongPtrW(hwnd,GWLP_USERDATA) as *const Data;
        if ptr.is_null() { return DefWindowProcW(hwnd,msg,w,l); }
        let data = &*ptr;
        match msg {
            UPDATE | WM_DISPLAYCHANGE | WM_DPICHANGED | WM_SETTINGCHANGE => {
                let snapshot = data.snapshot.lock().unwrap().clone();
                if !data.moving.load(Ordering::Relaxed) { if let Some(s) = snapshot.as_ref() { arrange(hwnd,s); } }
                InvalidateRect(hwnd,std::ptr::null(),0); 0
            }
            WM_TIMER => { let snapshot = data.snapshot.lock().unwrap().clone(); if !data.moving.load(Ordering::Relaxed) { if let Some(s) = snapshot.as_ref() { arrange(hwnd,s); } } 0 }
            WM_ERASEBKGND => 1,
            WM_PAINT => { let snapshot = data.snapshot.lock().unwrap().clone(); if let Some(s) = snapshot.as_ref() { paint(hwnd,s); } else { let mut ps = std::mem::zeroed(); BeginPaint(hwnd,&mut ps); EndPaint(hwnd,&ps); } 0 }
            WM_MOUSEACTIVATE => MA_NOACTIVATE as isize,
            WM_SETCURSOR => {
                let mut point: POINT = std::mem::zeroed(); GetCursorPos(&mut point); ScreenToClient(hwnd, &mut point);
                let scale = GetDpiForWindow(hwnd).max(96) as f64/96.0;
                if data.snapshot.lock().unwrap().as_ref().is_some_and(|s| badge_hit(s,scale,point.x,point.y)) {
                    SetCursor(LoadCursorW(null_mut(), IDC_HAND)); 1
                } else { DefWindowProcW(hwnd,msg,w,l) }
            }
            WM_NCHITTEST => {
                let mut rect: RECT = std::mem::zeroed(); GetWindowRect(hwnd,&mut rect);
                let x = (l as u32 & 0xffff) as i16 as i32;
                let unlocked = data.snapshot.lock().unwrap().as_ref().is_some_and(|s| !s.settings.mini_locked);
                if unlocked && x-rect.left < (16*GetDpiForWindow(hwnd).max(96)/96) as i32 { HTCAPTION as isize } else { HTCLIENT as isize }
            }
            WM_ENTERSIZEMOVE => { data.moving.store(true,Ordering::Relaxed); 0 }
            WM_EXITSIZEMOVE => {
                let mut rect: RECT = std::mem::zeroed(); GetWindowRect(hwnd,&mut rect);
                if let Some(s) = data.snapshot.lock().unwrap().as_mut() { s.settings.mini_x=Some(rect.left); s.settings.mini_y=Some(rect.top); }
                data.moving.store(false,Ordering::Relaxed);
                let app = data.app.clone(); let target = app.clone();
                let _ = app.run_on_main_thread(move || runtime::mini_action(&target,"position",Some((rect.left,rect.top)))); 0
            }
            WM_LBUTTONUP => {
                let x = (l as u32 & 0xffff) as i16 as i32;
                let y = ((l as u32 >> 16) & 0xffff) as i16 as i32;
                let scale = GetDpiForWindow(hwnd).max(96) as f64/96.0;
                let updates = data.snapshot.lock().unwrap().as_ref().is_some_and(|s| badge_hit(s,scale,x,y));
                let app = data.app.clone(); let target = app.clone();
                let _ = app.run_on_main_thread(move || if updates { runtime::show_updates(&target) } else { runtime::show_dashboard(&target) }); 0
            }
            WM_CONTEXTMENU => {
                let menu = CreatePopupMenu();
                let locked = data.snapshot.lock().unwrap().as_ref().map_or(true,|s| s.settings.mini_locked);
                for (id,label) in [(1,"사용량 / 미니바 설정"),(2,if locked { "위치 잠금 해제" } else { "위치 잠금" }),(3,"기본 위치로 이동"),(4,"미니바 숨기기")] {
                    AppendMenuW(menu,MF_STRING,id,wide(label).as_ptr());
                }
                let mut point: POINT = std::mem::zeroed(); GetCursorPos(&mut point);
                SetForegroundWindow(hwnd);
                let selected = TrackPopupMenu(menu,TPM_RETURNCMD|TPM_RIGHTBUTTON,point.x,point.y,0,hwnd,std::ptr::null());
                DestroyMenu(menu);
                let action = match selected { 1=>"show",2=>"lock",3=>"reset",4=>"hide",_=>"" };
                let app = data.app.clone(); let target = app.clone();
                let _ = app.run_on_main_thread(move || runtime::mini_action(&target,action,None)); 0
            }
            WM_CLOSE => {
                // Persist the close; otherwise the next timer would show it again.
                if let Some(s)=data.snapshot.lock().unwrap().as_mut() { s.settings.mini_enabled=false; }
                KillTimer(hwnd, 1);
                ShowWindow(hwnd,SW_HIDE);
                let app=data.app.clone(); let target=app.clone();
                let _=app.run_on_main_thread(move || runtime::mini_action(&target,"hide",None)); 0
            }
            WM_DESTROY => { PostQuitMessage(0); 0 }
            _ => DefWindowProcW(hwnd,msg,w,l)
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use crate::{model::{ProviderState, QuotaWindow, Settings}, updates::UpdateState};

        fn example(alert: bool) -> Snapshot {
            let mut updates = UpdateState::default();
            if alert { updates.available_version = Some("99.0.0".into()); updates.status = "error".into(); }
            Snapshot {
                providers: vec![ProviderState { id:"codex".into(),status:"ready".into(),message:String::new(),
                    windows:vec![QuotaWindow { label:"5시간".into(),used_percent:0.0,resets_at:None },
                                 QuotaWindow { label:"주간".into(),used_percent:21.0,resets_at:None }],
                    fetched_at:Some(now()),source:"예시".into() }],
                settings:Settings::default(),refreshing:false,claude_path:String::new(),bridge_command:String::new(),
                updates,running_providers:vec![],
            }
        }

        #[test]
        fn badge_hit_area_only_exists_for_visible_new_version() {
            let mut snapshot = example(true);
            for scale in [1.0,1.5] {
                let r = badge_rect(&snapshot,scale).unwrap();
                assert!(badge_hit(&snapshot,scale,r.left,r.top));
                assert!(badge_hit(&snapshot,scale,r.right-1,r.bottom-1));
                for (x,y) in [(r.left-1,r.top),(r.right,r.top),(r.left,r.top-1),(r.left,r.bottom)] {
                    assert!(!badge_hit(&snapshot,scale,x,y));
                }
                assert_eq!(r.right,((148-2) as f64*scale).round() as i32);
            }
            snapshot.updates.available_version=None;
            assert!(badge_rect(&snapshot,1.0).is_none());
            snapshot.updates.available_version=Some(snapshot.updates.current_version.clone());
            assert!(badge_rect(&snapshot,1.0).is_none());
            snapshot.updates.available_version=Some("99.0.0".into());
            snapshot.settings.mini_enabled=false;
            assert!(badge_rect(&snapshot,1.0).is_none());
            snapshot.settings.mini_enabled=true;
            snapshot.settings.mini_only_when_running=true;
            assert!(badge_rect(&snapshot,1.0).is_none());
            snapshot.running_providers.push("codex".into());
            assert!(badge_rect(&snapshot,1.0).is_some());
            snapshot.settings.mini_providers.push("claude".into());
            snapshot.running_providers.push("claude".into());
            assert_eq!(badge_rect(&snapshot,1.0).unwrap().right,278);
        }

        #[test]
        #[ignore = "set AI_TOKEN_BADGE_PREVIEW_DIR for offscreen GDI render previews"]
        fn render_badge_preview() {
            let directory = std::path::PathBuf::from(std::env::var_os("AI_TOKEN_BADGE_PREVIEW_DIR").expect("set AI_TOKEN_BADGE_PREVIEW_DIR"));
            assert!(directory.is_absolute(),"preview directory must be absolute");
            std::fs::create_dir_all(&directory).unwrap();
            for dpi in [96,144] {
                let scale=dpi as f64/96.0;
                let width=(148.0*scale).round() as i32;
                let height=(40.0*scale).round() as i32;
                for alert in [false,true] {
                    unsafe {
                        let dc=CreateCompatibleDC(null_mut()); assert!(!dc.is_null());
                        let bmi=BITMAPINFO { bmiHeader:BITMAPINFOHEADER { biSize:std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                            biWidth:width,biHeight:-height,biPlanes:1,biBitCount:32,biCompression:BI_RGB,..std::mem::zeroed() },..std::mem::zeroed() };
                        let mut pixels=std::ptr::null_mut();
                        let bitmap=CreateDIBSection(dc,&bmi,DIB_RGB_COLORS,&mut pixels,null_mut(),0);
                        assert!(!bitmap.is_null() && !pixels.is_null());
                        let old=SelectObject(dc,bitmap);
                        let snapshot=example(alert);
                        draw_contents(dc,&RECT { left:0,top:0,right:width,bottom:height },scale,&snapshot);
                        if alert {
                            let font=text_font(scale);
                            let old_font=SelectObject(dc,font);
                            let text=wide("5시간 100%"); let mut extent:SIZE=std::mem::zeroed();
                            assert_ne!(GetTextExtentPoint32W(dc,text.as_ptr(),(text.len()-1) as i32,&mut extent),0);
                            let available=(104.0*scale).round() as i32-(32.0*scale).round() as i32;
                            assert!(extent.cx<=available,"100% text at {dpi} DPI needs {}px but has {available}px",extent.cx);
                            SelectObject(dc,old_font); DeleteObject(font);
                        }
                        GdiFlush();
                        let source=std::slice::from_raw_parts(pixels as *const u8,(width*height*4) as usize);
                        let stride=((width*3+3)/4*4) as usize;
                        let size=stride*height as usize;
                        let mut bmp=Vec::with_capacity(54+size);
                        bmp.extend_from_slice(b"BM"); bmp.extend_from_slice(&((54+size) as u32).to_le_bytes());
                        bmp.extend_from_slice(&[0;4]); bmp.extend_from_slice(&54u32.to_le_bytes());
                        bmp.extend_from_slice(&40u32.to_le_bytes()); bmp.extend_from_slice(&width.to_le_bytes());
                        bmp.extend_from_slice(&(-height).to_le_bytes()); bmp.extend_from_slice(&1u16.to_le_bytes());
                        bmp.extend_from_slice(&24u16.to_le_bytes()); bmp.extend_from_slice(&0u32.to_le_bytes());
                        bmp.extend_from_slice(&(size as u32).to_le_bytes());
                        let ppm=(dpi as f64/0.0254).round() as i32;
                        bmp.extend_from_slice(&ppm.to_le_bytes()); bmp.extend_from_slice(&ppm.to_le_bytes()); bmp.extend_from_slice(&[0;8]);
                        for row in 0..height as usize {
                            let start=bmp.len();
                            for column in 0..width as usize {
                                let offset=(row*width as usize+column)*4;
                                bmp.extend_from_slice(&source[offset..offset+3]);
                            }
                            bmp.resize(start+stride,0);
                        }
                        let path=directory.join(format!("minibar-{dpi}-{}.bmp",if alert { "update" } else { "plain" }));
                        std::fs::write(path,bmp).unwrap();
                        SelectObject(dc,old); DeleteObject(bitmap); DeleteDC(dc);
                    }
                }
            }
        }
    }
}

#[cfg(windows)]
pub use platform::MiniBar;

#[cfg(not(windows))]
pub struct MiniBar;
#[cfg(not(windows))]
impl MiniBar {
    pub fn new(_: tauri::AppHandle) -> Self { Self }
    pub fn update(&self, _: &crate::model::Snapshot) {}
}
