pub fn escape_latex(input: &str) -> String {
    let mut escaped = String::with_capacity(input.len());
    for character in input.chars() {
        escaped.push_str(match character {
            '\\' => r"\textbackslash{}",
            '{' => r"\{",
            '}' => r"\}",
            '$' => r"\$",
            '&' => r"\&",
            '%' => r"\%",
            '#' => r"\#",
            '_' => r"\_",
            '^' => r"\textasciicircum{}",
            '~' => r"\textasciitilde{}",
            _ => {
                escaped.push(character);
                continue;
            }
        });
    }
    escaped
}
