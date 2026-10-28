import { copyFileSync, existsSync, lstatSync, mkdirSync, mkdtempSync, realpathSync, rmSync } from 'node:fs';
import { dirname, isAbsolute, join, relative, resolve, sep } from 'node:path';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

const projectRoot = realpathSync(resolve(dirname(fileURLToPath(import.meta.url)), '..'));
const source = join(projectRoot, 'public', 'app-icon.svg');
const cli = join(projectRoot, 'node_modules', '@tauri-apps', 'cli', 'tauri.js');
const icons = join(projectRoot, 'src-tauri', 'icons');
const tools = join(projectRoot, '.tools');
const keep = ['32x32.png', '128x128.png', '128x128@2x.png', 'icon.png', 'icon.ico', 'icon.icns'];

function insideProject(path) {
  const part = relative(projectRoot, path);
  return part !== '' && part !== '..' && !part.startsWith(`..${sep}`) && !isAbsolute(part);
}

if (!existsSync(source) || !existsSync(cli)) {
  throw new Error('App icon SVG or local Tauri CLI is missing. Run npm ci before generating icons.');
}
mkdirSync(tools, { recursive: true });
if (lstatSync(tools).isSymbolicLink() || realpathSync(tools) !== tools || !insideProject(tools)) {
  throw new Error('Refusing to generate icons inside a linked or external tools directory.');
}
const temporary = realpathSync(mkdtempSync(join(tools, 'icons-')));

try {
  const result = spawnSync(process.execPath, [cli, 'icon', source, '--output', temporary], {
    cwd: projectRoot,
    stdio: 'inherit',
    windowsHide: true,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`Tauri icon generation failed (${result.status ?? result.signal}).`);

  // Some CLI versions omit this conventional alias. Reuse the generated 256px PNG.
  if (!existsSync(join(temporary, 'icon.png'))) {
    copyFileSync(join(temporary, '128x128@2x.png'), join(temporary, 'icon.png'));
  }
  for (const name of keep) {
    const input = join(temporary, name);
    if (!lstatSync(input).isFile() || lstatSync(input).isSymbolicLink()) {
      throw new Error(`Expected generated icon is missing or linked: ${name}`);
    }
  }
  mkdirSync(icons, { recursive: true });
  if (lstatSync(icons).isSymbolicLink() || realpathSync(icons) !== icons || !insideProject(icons)) {
    throw new Error('Refusing to replace icons inside a linked or external directory.');
  }
  for (const name of keep) copyFileSync(join(temporary, name), join(icons, name));
  console.log(`Generated ${keep.length} desktop icons from public/app-icon.svg.`);
} finally {
  // Only remove this invocation's absolute, direct child of the checked .tools path.
  if (!isAbsolute(temporary) || dirname(temporary) !== tools || !insideProject(temporary)
      || lstatSync(temporary).isSymbolicLink() || realpathSync(temporary) !== temporary) {
    throw new Error('Refusing to remove an unexpected icon generation directory.');
  }
  rmSync(temporary, { recursive: true, force: true });
}
