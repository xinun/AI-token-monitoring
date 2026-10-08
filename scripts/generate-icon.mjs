import { deflateSync } from 'node:zlib';
import { writeFileSync } from 'node:fs';
const size = 256;
const pixels = Buffer.alloc((size * 4 + 1) * size);
for (let y = 0; y < size; y++) {
  for (let x = 0; x < size; x++) {
    const at = y * (size * 4 + 1) + 1 + x * 4;
    const dx = x - 127.5, dy = y - 127.5;
    const inside = dx * dx + dy * dy < 115 ** 2;
    const inner = dx * dx + dy * dy < 73 ** 2;
    const fill = inside ? inner && dx < 0 ? [227, 239, 203, 255] : [59, 82, 47, 255] : [0, 0, 0, 0];
    pixels.set(fill, at);
  }
}
function crc32(data) {
  let crc = 0xffffffff;
  for (const byte of data) { crc ^= byte; for (let i = 0; i < 8; i++) crc = (crc >>> 1) ^ (crc & 1 ? 0xedb88320 : 0); }
  return (crc ^ 0xffffffff) >>> 0;
}
function chunk(type, data) {
  const tag = Buffer.from(type), out = Buffer.alloc(data.length + 12);
  out.writeUInt32BE(data.length); tag.copy(out, 4); data.copy(out, 8); out.writeUInt32BE(crc32(Buffer.concat([tag, data])), data.length + 8);
  return out;
}
const header = Buffer.alloc(13); header.writeUInt32BE(size); header.writeUInt32BE(size, 4); header[8] = 8; header[9] = 6;
writeFileSync('app-icon.png', Buffer.concat([Buffer.from([137,80,78,71,13,10,26,10]), chunk('IHDR', header), chunk('IDAT', deflateSync(pixels)), chunk('IEND', Buffer.alloc(0))]));
