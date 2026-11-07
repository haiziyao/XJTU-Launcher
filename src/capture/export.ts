/**
 * Minimal PDF and ZIP writers for capture exports.
 *
 * Derived from XJTU-PPT-Capture-Extension v3.1.0 (MIT, Copyright (c) 2026
 * MeowSaint and contributors); see THIRD_PARTY_NOTICES.md.
 *
 * The upstream extension hand-builds both formats; this port keeps the same
 * approach so the plugin adds no dependency. JPEGs from the canvas are stored
 * as-is: ZIP uses the store method, PDF embeds them with /DCTDecode.
 */
export interface PdfPage {
  data: Uint8Array;
  width: number;
  height: number;
}

export interface ZipEntry {
  name: string;
  data: Uint8Array;
}

const encoder = new TextEncoder();

class ByteWriter {
  parts: Uint8Array[] = [];
  length = 0;

  bytes(value: Uint8Array): void {
    this.parts.push(value);
    this.length += value.length;
  }

  text(value: string): void {
    this.bytes(encoder.encode(value));
  }

  finish(): Uint8Array {
    const out = new Uint8Array(this.length);
    let offset = 0;
    for (const part of this.parts) {
      out.set(part, offset);
      offset += part.length;
    }
    return out;
  }
}

const CRC_TABLE = (() => {
  const table = new Uint32Array(256);
  for (let index = 0; index < 256; index += 1) {
    let value = index;
    for (let bit = 0; bit < 8; bit += 1) {
      value = value & 1 ? 0xedb88320 ^ (value >>> 1) : value >>> 1;
    }
    table[index] = value >>> 0;
  }
  return table;
})();

export function crc32(data: Uint8Array): number {
  let crc = 0xffffffff;
  for (const byte of data) crc = CRC_TABLE[(crc ^ byte) & 0xff] ^ (crc >>> 8);
  return (crc ^ 0xffffffff) >>> 0;
}

// Fixed 1980-01-01: the archive carries the page order, so a wall-clock
// stamp would only make identical exports differ.
const DOS_TIME = 0;
const DOS_DATE = (0 << 9) | (1 << 5) | 1;

/** Store-only ZIP (method 0): JPEGs are already compressed. */
export function buildZip(entries: ZipEntry[]): Uint8Array {
  if (!entries.length) throw new Error('没有可导出的图片');
  const time = DOS_TIME;
  const date = DOS_DATE;
  const local: Uint8Array[] = [];
  const central: Uint8Array[] = [];
  let offset = 0;
  for (const entry of entries) {
    const name = encoder.encode(entry.name);
    const crc = crc32(entry.data);
    const header = new Uint8Array(30 + name.length);
    const view = new DataView(header.buffer);
    view.setUint32(0, 0x04034b50, true);
    view.setUint16(4, 20, true);
    view.setUint16(6, 0x0800, true); // UTF-8 names
    view.setUint16(8, 0, true); // store
    view.setUint16(10, time, true);
    view.setUint16(12, date, true);
    view.setUint32(14, crc, true);
    view.setUint32(18, entry.data.length, true);
    view.setUint32(22, entry.data.length, true);
    view.setUint16(26, name.length, true);
    header.set(name, 30);
    local.push(header, entry.data);

    const directory = new Uint8Array(46 + name.length);
    const directoryView = new DataView(directory.buffer);
    directoryView.setUint32(0, 0x02014b50, true);
    directoryView.setUint16(4, 20, true);
    directoryView.setUint16(6, 20, true);
    directoryView.setUint16(8, 0x0800, true);
    directoryView.setUint16(10, 0, true);
    directoryView.setUint16(12, time, true);
    directoryView.setUint16(14, date, true);
    directoryView.setUint32(16, crc, true);
    directoryView.setUint32(20, entry.data.length, true);
    directoryView.setUint32(24, entry.data.length, true);
    directoryView.setUint16(28, name.length, true);
    directoryView.setUint32(42, offset, true);
    directory.set(name, 46);
    central.push(directory);

    offset += header.length + entry.data.length;
  }
  const directorySize = central.reduce((total, part) => total + part.length, 0);
  const end = new Uint8Array(22);
  const endView = new DataView(end.buffer);
  endView.setUint32(0, 0x06054b50, true);
  endView.setUint16(8, entries.length, true);
  endView.setUint16(10, entries.length, true);
  endView.setUint32(12, directorySize, true);
  endView.setUint32(16, offset, true);

  const out = new Uint8Array(offset + directorySize + end.length);
  let cursor = 0;
  for (const part of [...local, ...central, end]) {
    out.set(part, cursor);
    cursor += part.length;
  }
  return out;
}

/** One JPEG per page, page size follows the image aspect at 96 dpi. */
export function buildPdf(pages: PdfPage[]): Uint8Array {
  if (!pages.length) throw new Error('没有可导出的图片');
  const writer = new ByteWriter();
  const offsets: number[] = [];
  const object = (number: number, body: () => void) => {
    offsets[number] = writer.length;
    writer.text(`${number} 0 obj\n`);
    body();
    writer.text('\nendobj\n');
  };
  const stream = (data: Uint8Array, header: string) => {
    writer.text(`<< ${header} /Length ${data.length} >>\nstream\n`);
    writer.bytes(data);
    writer.text('\nendstream');
  };

  writer.text('%PDF-1.4\n');
  writer.bytes(new Uint8Array([0x25, 0xe2, 0xe3, 0xcf, 0xd3, 0x0a]));

  const kids = pages.map((_, index) => `${3 + index * 3} 0 R`).join(' ');
  object(1, () => writer.text('<< /Type /Catalog /Pages 2 0 R >>'));
  object(2, () => writer.text(`<< /Type /Pages /Kids [${kids}] /Count ${pages.length} >>`));

  pages.forEach((page, index) => {
    const pageObject = 3 + index * 3;
    const imageObject = pageObject + 1;
    const contentObject = pageObject + 2;
    const width = Math.max(1, Math.round(page.width * 0.75));
    const height = Math.max(1, Math.round(page.height * 0.75));
    object(pageObject, () =>
      writer.text(
        `<< /Type /Page /Parent 2 0 R /MediaBox [0 0 ${width} ${height}] ` +
          `/Resources << /XObject << /Im0 ${imageObject} 0 R >> >> ` +
          `/Contents ${contentObject} 0 R >>`,
      ),
    );
    object(imageObject, () =>
      stream(
        page.data,
        `/Type /XObject /Subtype /Image /Width ${page.width} /Height ${page.height} ` +
          '/ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode',
      ),
    );
    const content = encoder.encode(`q\n${width} 0 0 ${height} 0 0 cm\n/Im0 Do\nQ\n`);
    object(contentObject, () => {
      writer.text(`<< /Length ${content.length} >>\nstream\n`);
      writer.bytes(content);
      writer.text('endstream');
    });
  });

  const size = offsets.length;
  const xref = writer.length;
  writer.text(`xref\n0 ${size}\n0000000000 65535 f \n`);
  for (let number = 1; number < size; number += 1) {
    writer.text(`${String(offsets[number]).padStart(10, '0')} 00000 n \n`);
  }
  writer.text(`trailer\n<< /Size ${size} /Root 1 0 R >>\nstartxref\n${xref}\n%%EOF\n`);
  return writer.finish();
}

/**
 * A4 竖版讲义 PDF: two slides per page, each inside a 310×210 box at x=40,
 * with eight ruled lines to the right for handwritten notes. Geometry follows
 * the upstream 讲义 layout; the footer carries only ASCII so the PDF needs no
 * embedded CJK font.
 */
export function buildNotesPdf(pages: PdfPage[]): Uint8Array {
  if (!pages.length) throw new Error('没有可导出的图片');
  const PAGE_WIDTH = 595;
  const PAGE_HEIGHT = 842;
  const BOX_X = 40;
  const BOX_WIDTH = 310;
  const BOX_HEIGHT = 210;
  const RULE_X1 = 360;
  const RULE_X2 = 555;
  const bottoms = [526, 132];
  const num = (value: number) => value.toFixed(2);

  const writer = new ByteWriter();
  const offsets: number[] = [];
  const object = (number: number, body: () => void) => {
    offsets[number] = writer.length;
    writer.text(`${number} 0 obj\n`);
    body();
    writer.text('\nendobj\n');
  };
  const stream = (number: number, header: string, data: Uint8Array) => {
    object(number, () => {
      writer.text(`<< ${header} /Length ${data.length} >>\nstream\n`);
      writer.bytes(data);
      writer.text('\nendstream');
    });
  };

  const sheets = Math.ceil(pages.length / 2);
  const layout: { page: number; content: number; images: number[] }[] = [];
  let next = 4;
  for (let sheet = 0; sheet < sheets; sheet += 1) {
    const count = Math.min(2, pages.length - sheet * 2);
    const images: number[] = [];
    for (let slot = 0; slot < count; slot += 1) images.push(next++);
    layout.push({ page: next++, content: next++, images });
  }

  writer.text('%PDF-1.4\n');
  writer.bytes(new Uint8Array([0x25, 0xe2, 0xe3, 0xcf, 0xd3, 0x0a]));
  object(1, () => writer.text('<< /Type /Catalog /Pages 2 0 R >>'));
  object(2, () =>
    writer.text(
      `<< /Type /Pages /Kids [${layout.map((sheet) => `${sheet.page} 0 R`).join(' ')}] ` +
        `/Count ${sheets} >>`,
    ),
  );
  object(3, () => writer.text('<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>'));

  layout.forEach((sheet, sheetIndex) => {
    const start = sheetIndex * 2;
    const group = pages.slice(start, start + 2);
    const xobjects = group.map((_, slot) => `/Im${slot} ${sheet.images[slot]} 0 R`).join(' ');
    object(sheet.page, () =>
      writer.text(
        `<< /Type /Page /Parent 2 0 R /MediaBox [0 0 ${PAGE_WIDTH} ${PAGE_HEIGHT}] ` +
          `/Resources << /XObject << ${xobjects} >> /Font << /F1 3 0 R >> >> ` +
          `/Contents ${sheet.content} 0 R >>`,
      ),
    );
    const parts: string[] = [];
    group.forEach((image, slot) => {
      const bottom = bottoms[slot];
      const scale = Math.min(BOX_WIDTH / image.width, BOX_HEIGHT / image.height);
      const width = image.width * scale;
      const height = image.height * scale;
      const x = BOX_X + (BOX_WIDTH - width) / 2;
      const y = bottom + (BOX_HEIGHT - height) / 2;
      parts.push(`q\n${num(width)} 0 0 ${num(height)} ${num(x)} ${num(y)} cm\n/Im${slot} Do\nQ\n`);
      for (let line = 0; line < 8; line += 1) {
        const rule = bottom + 196 - line * 28;
        parts.push(`0.5 w\n${RULE_X1} ${rule} m ${RULE_X2} ${rule} l S\n`);
      }
      parts.push(`BT /F1 10 Tf ${BOX_X} ${bottom - 22} Td (${start + slot + 1}) Tj ET\n`);
    });
    parts.push(`BT /F1 10 Tf 280 24 Td (${sheetIndex + 1} / ${sheets}) Tj ET\n`);
    const content = encoder.encode(parts.join(''));
    object(sheet.content, () => {
      writer.text(`<< /Length ${content.length} >>\nstream\n`);
      writer.bytes(content);
      writer.text('\nendstream');
    });
    group.forEach((image, slot) =>
      stream(
        sheet.images[slot],
        `/Type /XObject /Subtype /Image /Width ${image.width} /Height ${image.height} ` +
          '/ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode',
        image.data,
      ),
    );
  });

  const size = offsets.length;
  const xref = writer.length;
  writer.text(`xref\n0 ${size}\n0000000000 65535 f \n`);
  for (let number = 1; number < size; number += 1) {
    writer.text(`${String(offsets[number]).padStart(10, '0')} 00000 n \n`);
  }
  writer.text(`trailer\n<< /Size ${size} /Root 1 0 R >>\nstartxref\n${xref}\n%%EOF\n`);
  return writer.finish();
}
