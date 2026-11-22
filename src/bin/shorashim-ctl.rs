//! `shorashim-ctl`: console companion of Шорашим for automation.
//! `shorashim-ctl state`, `shorashim-ctl press label=Начать`,
//! `shorashim-ctl mcp`, `shorashim-ctl --headless --script s.jsonl`.

fn main() {
    std::process::exit(shorashim::automation::cli::companion_main());
}
