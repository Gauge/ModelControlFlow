use std::fs::OpenOptions;
use std::io::Write;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let path = args.get(1).cloned().unwrap_or_default();
    let mark = args.get(2).cloned().unwrap_or_default();
    let count: usize = args.get(3).and_then(|n| n.parse().ok()).unwrap_or(2000);
    let size: usize = args.get(4).and_then(|n| n.parse().ok()).unwrap_or(400);

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .expect("the file opens");
    let filler = mark.repeat(size / mark.len().max(1));
    for index in 0..count {
        let line = format!("{mark} {index} {filler}\n");
        file.write_all(line.as_bytes()).expect("it writes");
    }
}
