/// Strip non-SGR ANSI escape sequences from raw bytes.
///
/// Preserves SGR sequences (CSI ... m) which control text styling (colors, bold, etc.)
/// and are safe for `ansi-to-tui` parsing. Strips:
/// - OSC sequences (\x1b] ... ST) — title set, clipboard write, etc.
/// - DCS sequences (\x1bP ... ST) — device control, Sixel graphics
/// - Other CSI sequences that are not SGR (CSI ... <anything except 'm'>)
///
/// The string terminator (ST) can be either \x1b\\ (ESC \\) or \x07 (BEL).
pub fn sanitize_ansi(raw: &[u8]) -> Vec<u8> {
    let mut result = Vec::with_capacity(raw.len());
    let mut i = 0;

    while i < raw.len() {
        if raw[i] != 0x1b {
            result.push(raw[i]);
            i += 1;
            continue;
        }

        // We found ESC (0x1b)
        if i + 1 >= raw.len() {
            // ESC at end of input — drop it
            i += 1;
            continue;
        }

        match raw[i + 1] {
            // OSC: ESC ] ... (ST or BEL)
            b']' => {
                i = skip_until_st(raw, i + 2);
            }
            // DCS: ESC P ... (ST or BEL)
            b'P' => {
                i = skip_until_st(raw, i + 2);
            }
            // CSI: ESC [ ... <final byte>
            b'[' => {
                let start = i;
                i += 2; // skip ESC [

                // Collect parameter bytes (0x30-0x3F) and intermediate bytes (0x20-0x2F)
                while i < raw.len() && raw[i] >= 0x20 && raw[i] <= 0x3f {
                    i += 1;
                }
                // Intermediate bytes
                while i < raw.len() && raw[i] >= 0x20 && raw[i] <= 0x2f {
                    i += 1;
                }

                if i < raw.len() {
                    let final_byte = raw[i];
                    i += 1;

                    if final_byte == b'm' {
                        // SGR — keep it
                        result.extend_from_slice(&raw[start..i]);
                    }
                    // Non-SGR CSI — dropped
                }
            }
            // Other ESC sequences (e.g., ESC c for RIS) — drop the two-byte sequence
            _ => {
                i += 2;
            }
        }
    }

    result
}

/// Skip bytes until a String Terminator is found: ESC \\ or BEL (0x07).
fn skip_until_st(raw: &[u8], start: usize) -> usize {
    let mut i = start;
    while i < raw.len() {
        if raw[i] == 0x07 {
            // BEL terminates
            return i + 1;
        }
        if raw[i] == 0x1b && i + 1 < raw.len() && raw[i + 1] == b'\\' {
            // ESC \ terminates
            return i + 2;
        }
        i += 1;
    }
    // No terminator found — skip everything
    i
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_plain_text() {
        let input = b"Hello, world!";
        assert_eq!(sanitize_ansi(input), input.to_vec());
    }

    #[test]
    fn preserves_sgr_sequences() {
        // ESC[31m = red foreground, ESC[0m = reset
        let input = b"\x1b[31mRed text\x1b[0m";
        assert_eq!(sanitize_ansi(input), input.to_vec());
    }

    #[test]
    fn strips_osc_52_clipboard_write() {
        // OSC 52 clipboard write: ESC ] 52 ; c ; <base64> ST
        let input = b"before\x1b]52;c;SGVsbG8=\x1b\\after";
        let expected = b"beforeafter";
        assert_eq!(sanitize_ansi(input), expected.to_vec());
    }

    #[test]
    fn strips_osc_0_title_set() {
        // OSC 0 title set: ESC ] 0 ; title BEL
        let input = b"before\x1b]0;My Title\x07after";
        let expected = b"beforeafter";
        assert_eq!(sanitize_ansi(input), expected.to_vec());
    }

    #[test]
    fn strips_dcs_sequence() {
        // DCS: ESC P ... ST
        let input = b"before\x1bPsome-dcs-data\x1b\\after";
        let expected = b"beforeafter";
        assert_eq!(sanitize_ansi(input), expected.to_vec());
    }

    #[test]
    fn strips_csi_cursor_movement() {
        // CSI H (cursor home) should be stripped
        let input = b"before\x1b[Hafter";
        let expected = b"beforeafter";
        assert_eq!(sanitize_ansi(input), expected.to_vec());
    }

    #[test]
    fn preserves_sgr_with_parameters() {
        // ESC[38;5;196m = 256-color red
        let input = b"\x1b[38;5;196mcolored\x1b[0m";
        assert_eq!(sanitize_ansi(input), input.to_vec());
    }

    #[test]
    fn preserves_sgr_rgb() {
        // ESC[38;2;255;0;128m = RGB color
        let input = b"\x1b[38;2;255;0;128mrgb\x1b[0m";
        assert_eq!(sanitize_ansi(input), input.to_vec());
    }

    #[test]
    fn handles_empty_input() {
        assert_eq!(sanitize_ansi(b""), Vec::<u8>::new());
    }

    #[test]
    fn handles_trailing_esc() {
        let input = b"text\x1b";
        let expected = b"text";
        assert_eq!(sanitize_ansi(input), expected.to_vec());
    }

    #[test]
    fn mixed_sgr_and_non_sgr() {
        // SGR (keep) + cursor position (strip) + SGR (keep)
        let input = b"\x1b[1mbold\x1b[10;20Hmoved\x1b[0mreset";
        let expected = b"\x1b[1mboldmoved\x1b[0mreset";
        assert_eq!(sanitize_ansi(input), expected.to_vec());
    }
}
