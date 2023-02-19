use std::fs;
use std::io;
use std::cmp;
use std::env;
use std::time;
use rand::seq::SliceRandom;

use crossterm::{
    cursor,
    terminal,
    QueueableCommand
};

fn read_token_list(file: &str) -> Vec<Vec<String>> {
    let character_content = fs::read_to_string(file).expect("Couldn't open character file");
    let tokens = character_content.split("\n").collect::<Vec<_>>();
    let longest = tokens
        .iter()
        .max_by(|&s0, &s1| s0.len().cmp(&s1.len()))
        .expect("Empty file not allowed").len();

    let mut token_list = vec![Vec::new(); longest];
    for &token in &tokens {
        if token.len() == 0 {
            continue;
        }

        token_list[token.len() - 1].push(token.to_owned());
    }

    token_list
}

fn read_image(file: &str) -> gif::Decoder<fs::File> {
    let input_file = fs::File::open(file).expect("Couldn't open gif file");
    let mut options = gif::DecodeOptions::new();
    options.set_color_output(gif::ColorOutput::RGBA);
    options.read_info(input_file).expect("Couldn't read gif file info")
}

fn is_white(frame: &gif::Frame, x: u16, y: u16, sx: u16, sy: u16) -> bool {
    let base_x = x - frame.left;
    let base_y = y - frame.top;

    // take average of square
    let mut sum = 0u32;
    for lx in base_x..std::cmp::min(base_x + sx, frame.width) {
        for ly in base_y..std::cmp::min(base_y + sy, frame.height) {
            let index = (ly as usize * frame.width as usize + lx as usize) * 4;
            let r = frame.buffer[index + 0];
            let g = frame.buffer[index + 1];
            let b = frame.buffer[index + 2];
            sum += cmp::max(cmp::max(r, g), b) as u32;
        }
    }

    (sum / (sx * sy) as u32) > 50
}

fn term_clear(stdout: &mut impl io::Write) {
    stdout.write(format!("{esc}[2J{esc}[1;1H", esc = 27 as char).as_bytes());
    stdout.flush();
}

fn draw_frame(stdout: &mut impl io::Write, tokens: &Vec<Vec<String>>, frame: &gif::Frame, sx: u16, sy: u16) {
    let frame_right = frame.left + frame.width;
    let frame_bot   = frame.top + frame.height;

    let mut x;
    let mut x_end;

    term_clear(stdout);
    for y in (frame.top..frame_bot).step_by(sy as usize) {
        // new row: reset x
        x = frame.left;
        x_end = x;

        while x < frame_right {
            // continue from where we left off
            x = x_end;

            // go to beginning of white
            while x < frame_right && !is_white(&frame, x, y, sx, sy) {
                x += sx;
            }

            if x >= frame_right {
                // no white found
                continue;
            }

            x_end = x;
            // x_end points one past the last white square
            // (also have to make sure row length does not exceed max token length)
            while x_end < frame_right && (x_end - x) / sx < tokens.len() as u16 && is_white(&frame, x_end, y, sx, sy) {
                x_end += sx;
            }

            let token_length = (x_end - x) / sx;
            let token = tokens[token_length as usize - 1].choose(&mut rand::thread_rng()).unwrap();
            stdout.queue(cursor::MoveTo(x / sx, y / sy));
            stdout.write(token.as_bytes());
            stdout.flush();
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();
    assert!(args.len() == 3, "Usage: ascii-render CHARACTER_FILE GIF");

    let token_list = read_token_list(&args[1]);

    let mut stdout = io::stdout();
    let mut decoder = read_image(&args[2]);

    let mut sx = 0;
    let mut sy = 0;
    while let Some(frame) = decoder.read_next_frame().unwrap() {
        if sx == 0 {
            // need to get image size from first frame
            // didn't find a better way unfortunately :(
            let (tx, ty) = terminal::size().expect("Couldn't query terminal size");
            sx = frame.width / tx;
            sy = frame.height / ty;
            assert!(frame.width >= tx, "Image needs to be at least as wide as the terminal");
            assert!(frame.height >= ty, "Image needs to be at least as tall as the terminal");
        }

        let start = time::Instant::now();
        draw_frame(&mut stdout, &token_list, &frame, sx, sy);
        let end = time::Instant::now();
        let delay = time::Duration::from_millis(frame.delay as u64 * 10);
        if start + delay > end {
            // still need to wait some time
            std::thread::sleep(start + delay - end);
        }
    }
}
