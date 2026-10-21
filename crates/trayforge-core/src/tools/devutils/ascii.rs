use figlet_rs::FIGfont;

use crate::{
    Category, OptionKind, Request, Tool, ToolError, ToolInfo, ToolOption, ToolOutput, ToolResult,
};

/// 5x7 glyphs, seven space-separated rows each; `#` is a lit pixel.
const GLYPHS: &[(char, &str)] = &[
    ('A', ".###. #...# #...# ##### #...# #...# #...#"),
    ('B', "####. #...# #...# ####. #...# #...# ####."),
    ('C', ".#### #.... #.... #.... #.... #.... .####"),
    ('D', "####. #...# #...# #...# #...# #...# ####."),
    ('E', "##### #.... #.... ####. #.... #.... #####"),
    ('F', "##### #.... #.... ####. #.... #.... #...."),
    ('G', ".#### #.... #.... #..## #...# #...# .####"),
    ('H', "#...# #...# #...# ##### #...# #...# #...#"),
    ('I', "##### ..#.. ..#.. ..#.. ..#.. ..#.. #####"),
    ('J', "..### ...#. ...#. ...#. ...#. #..#. .##.."),
    ('K', "#...# #..#. #.#.. ##... #.#.. #..#. #...#"),
    ('L', "#.... #.... #.... #.... #.... #.... #####"),
    ('M', "#...# ##.## #.#.# #.#.# #...# #...# #...#"),
    ('N', "#...# ##..# #.#.# #..## #...# #...# #...#"),
    ('O', ".###. #...# #...# #...# #...# #...# .###."),
    ('P', "####. #...# #...# ####. #.... #.... #...."),
    ('Q', ".###. #...# #...# #...# #.#.# #..#. .##.#"),
    ('R', "####. #...# #...# ####. #.#.. #..#. #...#"),
    ('S', ".#### #.... #.... .###. ....# ....# ####."),
    ('T', "##### ..#.. ..#.. ..#.. ..#.. ..#.. ..#.."),
    ('U', "#...# #...# #...# #...# #...# #...# .###."),
    ('V', "#...# #...# #...# #...# #...# .#.#. ..#.."),
    ('W', "#...# #...# #...# #.#.# #.#.# ##.## #...#"),
    ('X', "#...# #...# .#.#. ..#.. .#.#. #...# #...#"),
    ('Y', "#...# #...# .#.#. ..#.. ..#.. ..#.. ..#.."),
    ('Z', "##### ....# ...#. ..#.. .#... #.... #####"),
    ('0', ".###. #...# #..## #.#.# ##..# #...# .###."),
    ('1', "..#.. .##.. ..#.. ..#.. ..#.. ..#.. .###."),
    ('2', ".###. #...# ....# ...#. ..#.. .#... #####"),
    ('3', "####. ....# ....# .###. ....# ....# ####."),
    ('4', "...#. ..##. .#.#. #..#. ##### ...#. ...#."),
    ('5', "##### #.... ####. ....# ....# #...# .###."),
    ('6', "..##. .#... #.... ####. #...# #...# .###."),
    ('7', "##### ....# ...#. ..#.. .#... .#... .#..."),
    ('8', ".###. #...# #...# .###. #...# #...# .###."),
    ('9', ".###. #...# #...# .#### ....# ...#. .##.."),
    ('.', "..... ..... ..... ..... ..... .##.. .##.."),
    (',', "..... ..... ..... ..... .##.. ..#.. .#..."),
    ('!', "..#.. ..#.. ..#.. ..#.. ..#.. ..... ..#.."),
    ('?', ".###. #...# ....# ...#. ..#.. ..... ..#.."),
    ('-', "..... ..... ..... ##### ..... ..... ....."),
    ('_', "..... ..... ..... ..... ..... ..... #####"),
    (':', "..... .##.. .##.. ..... .##.. .##.. ....."),
    ('\'', "..#.. ..#.. .#... ..... ..... ..... ....."),
    ('/', "....# ....# ...#. ..#.. .#... #.... #...."),
    ('+', "..... ..#.. ..#.. ##### ..#.. ..#.. ....."),
    ('=', "..... ..... ##### ..... ##### ..... ....."),
    ('(', "...#. ..#.. .#... .#... .#... ..#.. ...#."),
    (')', ".#... ..#.. ...#. ...#. ...#. ..#.. .#..."),
    ('*', "..... #.#.# .###. ##### .###. #.#.# ....."),
];

const ROWS: usize = 7;
const SPACE_WIDTH: usize = 3;
const FONTS: [&str; 6] = ["Standard", "Banner", "Block", "Slant", "Small", "Mini"];

pub fn tool() -> Tool {
    Tool {
        info: ToolInfo {
            id: "ascii-art",
            name: "ASCII Art",
            category: Category::DevUtils,
            description: "Generate ASCII art text banners",
            keywords: vec!["ascii", "text", "banner", "figlet", "font"],
            icon: "type",
            input_placeholder: "Enter text to convert to ASCII art...",
            output_placeholder: "ASCII art will appear here",
            primary_action: "Generate",
            secondary_action: None,
            supports_file_input: false,
            allows_empty_input: false,
            custom_ui: false,
            options: vec![
                ToolOption {
                    id: "font",
                    label: "Font style",
                    help: Some("ASCII art font style"),
                    kind: OptionKind::Picker {
                        choices: FONTS.to_vec(),
                        default: 0,
                    },
                },
                ToolOption {
                    id: "alignment",
                    label: "Alignment",
                    help: Some("Text alignment"),
                    kind: OptionKind::Picker {
                        choices: vec!["Left", "Center", "Right"],
                        default: 0,
                    },
                },
                ToolOption {
                    id: "width",
                    label: "Max width",
                    help: Some("Maximum output width"),
                    kind: OptionKind::Stepper {
                        min: 20,
                        max: 300,
                        default: 80,
                    },
                },
            ],
        },
        process: Some(process),
    }
}

fn process(req: &Request) -> ToolResult {
    let font = req.options.str("font");
    let max_width = req.options.i64("width") as usize;
    let figlet = (font == "Standard")
        .then(FIGfont::standard)
        .transpose()
        .map_err(ToolError::ProcessingFailed)?;
    let render = |text: &str| -> Vec<String> {
        match &figlet {
            Some(figlet) => figlet
                .convert(text)
                .map(|f| f.to_string().lines().map(str::to_string).collect())
                .unwrap_or_default(),
            None => render_bitmap(font, text),
        }
    };

    let mut lines = Vec::new();
    for input_line in req.input.lines() {
        let mut current: Vec<String> = Vec::new();
        let mut text = String::new();
        for word in input_line.split_whitespace() {
            let candidate = if text.is_empty() {
                word.to_string()
            } else {
                format!("{text} {word}")
            };
            let art = render(&candidate);
            if !text.is_empty() && width(&art) > max_width {
                lines.append(&mut current);
                lines.push(String::new());
                text = word.to_string();
                current = render(word);
            } else {
                text = candidate;
                current = art;
            }
        }
        lines.append(&mut current);
        lines.push(String::new());
    }

    let block_width = width(&lines);
    let aligned: Vec<String> = lines
        .iter()
        .map(|line| {
            let pad = match req.options.str("alignment") {
                "Center" => {
                    max_width
                        .min(block_width)
                        .saturating_sub(line.chars().count())
                        / 2
                }
                "Right" => max_width
                    .min(block_width)
                    .saturating_sub(line.chars().count()),
                _ => 0,
            };
            format!("{}{line}", " ".repeat(pad))
        })
        .collect();
    let art = aligned.join("\n").trim_end().to_string();
    if art.is_empty() {
        return Err(ToolError::ProcessingFailed(
            "Could not render the text with this font".into(),
        ));
    }
    Ok(ToolOutput::text(art))
}

fn width(lines: &[String]) -> usize {
    lines.iter().map(|l| l.chars().count()).max().unwrap_or(0)
}

/// Pixel grid (rows of lit flags) for `text`, one blank column between glyphs.
fn bitmap(text: &str) -> Vec<Vec<bool>> {
    let mut grid = vec![Vec::new(); ROWS];
    for c in text.chars() {
        let c = c.to_ascii_uppercase();
        let glyph = GLYPHS
            .iter()
            .find(|(g, _)| *g == c)
            .or_else(|| GLYPHS.iter().find(|(g, _)| *g == '?'));
        match glyph {
            Some((_, rows)) if c != ' ' => {
                for (grid_row, row) in grid.iter_mut().zip(rows.split(' ')) {
                    grid_row.extend(row.chars().map(|p| p == '#'));
                    grid_row.push(false);
                }
            }
            _ => grid.iter_mut().for_each(|r| r.extend([false; SPACE_WIDTH])),
        }
    }
    grid
}

fn render_bitmap(font: &str, text: &str) -> Vec<String> {
    let grid = bitmap(text);
    let columns = grid[0].len();
    let lit = |r: usize, c: usize| r < ROWS && c < columns && grid[r][c];
    let lines: Vec<String> = match font {
        "Block" => grid
            .iter()
            .map(|row| row.iter().map(|p| if *p { '█' } else { ' ' }).collect())
            .collect(),
        "Slant" => grid
            .iter()
            .enumerate()
            .map(|(r, row)| {
                let shift = " ".repeat((ROWS - 1 - r) / 2);
                let body: String = row.iter().map(|p| if *p { '#' } else { ' ' }).collect();
                format!("{shift}{body}")
            })
            .collect(),
        // Two pixel rows per text row.
        "Small" => (0..ROWS.div_ceil(2))
            .map(|r| {
                (0..columns)
                    .map(|c| match (lit(2 * r, c), lit(2 * r + 1, c)) {
                        (true, true) => '█',
                        (true, false) => '▀',
                        (false, true) => '▄',
                        (false, false) => ' ',
                    })
                    .collect()
            })
            .collect(),
        // Braille cells: 2 x 4 pixels each.
        "Mini" => (0..ROWS.div_ceil(4))
            .map(|r| {
                (0..columns.div_ceil(2))
                    .map(|c| {
                        const DOTS: [[u32; 4]; 2] =
                            [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];
                        let bits: u32 = (0..2)
                            .flat_map(|dx| (0..4).map(move |dy| (dx, dy)))
                            .filter(|(dx, dy)| lit(4 * r + dy, 2 * c + dx))
                            .map(|(dx, dy)| DOTS[dx][dy])
                            .sum();
                        char::from_u32(0x2800 + bits).unwrap_or(' ')
                    })
                    .collect()
            })
            .collect(),
        _ => grid
            .iter()
            .map(|row| row.iter().map(|p| if *p { '#' } else { ' ' }).collect())
            .collect(),
    };
    lines
        .into_iter()
        .map(|l: String| l.trim_end().to_string())
        .collect()
}
