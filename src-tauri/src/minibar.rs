//! Windows-only, event-driven GDI mini bar. No WebView or renderer process.
#[cfg(windows)]
mod platform {
    use crate::{model::{now, Snapshot}, runtime};
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
                // Sleep in GetMessage between updates; timer only checks position and stacking.
                SetTimer(hwnd, 1, 2000, None);
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
        let width = px(16 + 132 * s.settings.mini_providers.len() as i32);
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
        // Stay visible until the user disables the bar. Geometry/focus changes
        // must never turn a persisted enabled setting into an automatic hide/show cycle.
        let visible = s.settings.mini_enabled && !s.settings.mini_providers.is_empty();
        (x, y, width, height, visible)
    }

    unsafe fn arrange(hwnd: HWND, s: &Snapshot) {
        let (x,y,w,h,visible) = geometry(hwnd,s);
        let mut current: RECT = std::mem::zeroed(); GetWindowRect(hwnd, &mut current);
        if visible {
            if current.left != x || current.top != y || current.right-current.left != w || current.bottom-current.top != h || IsWindowVisible(hwnd) == 0 {
                SetWindowPos(hwnd, HWND_TOPMOST, x,y,w,h,SWP_NOACTIVATE | SWP_SHOWWINDOW);
            } else {
                // Explorer can raise its taskbar above a separate overlay. Restore
                // stacking without hiding, resizing, repainting, or taking focus.
                SetWindowPos(hwnd, HWND_TOPMOST,0,0,0,0,SWP_NOACTIVATE|SWP_NOMOVE|SWP_NOSIZE);
            }
        } else if IsWindowVisible(hwnd) != 0 { ShowWindow(hwnd, SW_HIDE); }
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

    unsafe fn paint(hwnd: HWND, s: &Snapshot) {
        let mut ps: PAINTSTRUCT = std::mem::zeroed(); let window_dc = BeginPaint(hwnd,&mut ps);
        let scale = GetDpiForWindow(hwnd).max(96) as f64/96.0;
        let px = |v: i32| (v as f64*scale).round() as i32;
        let mut area: RECT = std::mem::zeroed(); GetClientRect(hwnd,&mut area);
        let buffer = CreateCompatibleDC(window_dc);
        let bitmap = CreateCompatibleBitmap(window_dc,(area.right-area.left).max(1),(area.bottom-area.top).max(1));
        let buffered = !buffer.is_null() && !bitmap.is_null();
        let dc = if buffered { buffer } else { window_dc };
        let old_bitmap = if buffered { SelectObject(dc,bitmap) } else { null_mut() };
        let bg = CreateSolidBrush(BACKGROUND); FillRect(dc,&area,bg); DeleteObject(bg);
        SetBkMode(dc,TRANSPARENT as i32);
        let font = CreateFontW(-px(13),0,0,0,500,0,0,0,DEFAULT_CHARSET as u32,OUT_DEFAULT_PRECIS as u32,
            CLIP_DEFAULT_PRECIS as u32,ANTIALIASED_QUALITY as u32,DEFAULT_PITCH as u32,wide("Malgun Gothic").as_ptr());
        let old = SelectObject(dc,font);
        SetTextColor(dc,0x00889988);
        let mut grip = RECT { left:px(2),top:0,right:px(14),bottom:area.bottom };
        DrawTextW(dc,wide(if s.settings.mini_locked { "·" } else { "⋮" }).as_ptr(),-1,&mut grip,DT_CENTER|DT_VCENTER|DT_SINGLELINE);
        for (index,id) in s.settings.mini_providers.iter().enumerate() {
            let left = px(16+index as i32*132);
            logo(dc,id,left,px(8),px(24));
            let lines = crate::model::mini_lines(s.providers.iter().find(|p| &p.id==id), now());
            for (row,(text,low)) in lines.iter().enumerate() {
                SetTextColor(dc,if *low { 0x008ec0ff } else { 0x00ecf0ed });
                let mut r = RECT { left:left+px(32),top:px(2+row as i32*18),right:left+px(128),bottom:px(20+row as i32*18) };
                DrawTextW(dc,wide(text).as_ptr(),-1,&mut r,DT_LEFT|DT_VCENTER|DT_SINGLELINE|DT_END_ELLIPSIS);
            }
        }
        SelectObject(dc,old); DeleteObject(font);
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
                let app = data.app.clone(); let target = app.clone(); let _ = app.run_on_main_thread(move || runtime::show_dashboard(&target)); 0
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
                ShowWindow(hwnd,SW_HIDE);
                let app=data.app.clone(); let target=app.clone();
                let _=app.run_on_main_thread(move || runtime::mini_action(&target,"hide",None)); 0
            }
            WM_DESTROY => { PostQuitMessage(0); 0 }
            _ => DefWindowProcW(hwnd,msg,w,l)
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
