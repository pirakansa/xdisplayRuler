pub fn display_ruler() -> String {
    let tens = (0..8)
        .map(|index| format!("{index:<10}"))
        .collect::<String>();
    let ones = "1234567890".repeat(8);

    format!("{tens}\n{ones}")
}

#[cfg(test)]
mod tests {
    use super::display_ruler;

    #[test]
    fn builds_an_80_column_ruler() {
        let ruler = display_ruler();
        let mut lines = ruler.lines();

        assert_eq!(lines.next(), Some("0         1         2         3         4         5         6         7"));
        assert_eq!(lines.next(), Some("12345678901234567890123456789012345678901234567890123456789012345678901234567890"));
        assert_eq!(lines.next(), None);
    }
}