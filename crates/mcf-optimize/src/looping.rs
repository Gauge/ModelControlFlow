const ENOUGH_REPEATS: usize = 12;
const SHORTEST_CYCLE: usize = 8;
const LONGEST_CYCLE: usize = 200;
const CYCLES_BEFORE_CALLING_IT: usize = 6;
const LOOKED_AT: usize = 4000;

#[must_use]
pub fn looping(said: &str) -> Option<String> {
    let tail = said.get(said.len().saturating_sub(LOOKED_AT)..).unwrap_or(said);
    if let Some(found) = a_line_over_and_over(tail) {
        return Some(found);
    }
    a_cycle_at_the_end(tail)
}

fn a_line_over_and_over(tail: &str) -> Option<String> {
    let mut counted: Vec<(&str, usize)> = Vec::new();
    for line in tail.lines().map(str::trim).filter(|line| line.len() > 15) {
        if let Some(found) = counted.iter_mut().find(|(held, _)| *held == line) {
            found.1 = found.1.saturating_add(1);
        } else {
            counted.push((line, 1));
        }
    }
    let (line, seen) = counted.into_iter().max_by_key(|(_, seen)| *seen)?;
    (seen >= ENOUGH_REPEATS).then(|| {
        let shown: String = line.chars().take(40).collect();
        format!("a line {seen} times over: {shown}")
    })
}

fn a_cycle_at_the_end(tail: &str) -> Option<String> {
    let end = tail.get(tail.len().saturating_sub(800)..).unwrap_or(tail);
    let bytes = end.as_bytes();
    for length in SHORTEST_CYCLE..=LONGEST_CYCLE {
        let wanted = length.checked_mul(CYCLES_BEFORE_CALLING_IT)?;
        if bytes.len() < wanted {
            break;
        }
        let Some(unit) = bytes.get(bytes.len().saturating_sub(length)..) else {
            continue;
        };
        let repeated = (0..CYCLES_BEFORE_CALLING_IT).all(|back| {
            let from = bytes.len().saturating_sub(length.saturating_mul(back + 1));
            let to = bytes.len().saturating_sub(length.saturating_mul(back));
            bytes.get(from..to) == Some(unit)
        });
        if repeated {
            return Some(format!(
                "{length} characters {CYCLES_BEFORE_CALLING_IT} times over"
            ));
        }
    }
    None
}

#[cfg(test)]
mod tests;
