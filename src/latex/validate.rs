use std::collections::BTreeSet;

pub fn validate_generated_latex(latex: &str) -> Result<(), String> {
    if [
        "{{PACKAGES}}",
        "{{TITLE_AUTHOR}}",
        "{{ABSTRACT}}",
        "{{KEYWORDS}}",
        "{{BODY}}",
        "{{REFERENCES}}",
    ]
    .iter()
    .any(|marker| latex.contains(marker))
    {
        return Err("an internal template marker was not replaced".to_owned());
    }

    let bytes = latex.as_bytes();
    let mut index = 0;
    let mut brace_depth = 0usize;
    let mut environments = Vec::new();
    let mut citations = BTreeSet::new();
    let mut bibliography_items = BTreeSet::new();

    while index < bytes.len() {
        match bytes[index] {
            b'%' => {
                while index < bytes.len() && bytes[index] != b'\n' {
                    index += 1;
                }
            }
            b'{' => {
                brace_depth += 1;
                index += 1;
            }
            b'}' => {
                if brace_depth == 0 {
                    return Err("a closing brace has no matching opening brace".to_owned());
                }
                brace_depth -= 1;
                index += 1;
            }
            b'\\' => {
                index += 1;
                if index == bytes.len() {
                    return Err("the document ends with an incomplete command".to_owned());
                }
                let command_start = index;
                if bytes[index].is_ascii_alphabetic() {
                    while index < bytes.len() && bytes[index].is_ascii_alphabetic() {
                        index += 1;
                    }
                } else {
                    index += 1;
                }
                let command = std::str::from_utf8(&bytes[command_start..index])
                    .map_err(|_| "the document contains an invalid command".to_owned())?;

                if command == "verb" {
                    if bytes.get(index) == Some(&b'*') {
                        index += 1;
                    }
                    let Some(&delimiter) = bytes.get(index) else {
                        return Err("an inline verbatim command is incomplete".to_owned());
                    };
                    index += 1;
                    let Some(end) = bytes[index..].iter().position(|byte| *byte == delimiter)
                    else {
                        return Err(
                            "an inline verbatim command has no closing delimiter".to_owned()
                        );
                    };
                    index += end + 1;
                    continue;
                }

                if is_external_file_command(command) {
                    return Err(format!("\\{command} references a separate content file"));
                }

                if command == "begin" || command == "end" {
                    let Some((environment, end)) = read_braced_group(bytes, index) else {
                        return Err(format!("\\{command} has no environment name"));
                    };
                    let environment = environment.trim();
                    if command == "begin" && is_verbatim_environment(environment) {
                        let marker = format!("\\end{{{environment}}}");
                        let Some(offset) = bytes[end..]
                            .windows(marker.len())
                            .position(|window| window == marker.as_bytes())
                        else {
                            return Err(format!("environment `{environment}` is not closed"));
                        };
                        index = end + offset + marker.len();
                        continue;
                    }
                    if command == "begin" {
                        environments.push(environment.to_owned());
                    } else if environments.pop().as_deref() != Some(environment) {
                        return Err(format!("environment `{environment}` closes out of order"));
                    }
                    index = end;
                    continue;
                }

                if is_citation_command(command) || command == "bibitem" {
                    let Some((keys, end)) = read_braced_group(bytes, index) else {
                        return Err(format!("\\{command} has no key list"));
                    };
                    let destination = if command == "bibitem" {
                        &mut bibliography_items
                    } else {
                        &mut citations
                    };
                    destination.extend(
                        keys.split(',')
                            .map(str::trim)
                            .filter(|key| !key.is_empty())
                            .map(str::to_owned),
                    );
                    index = end;
                    continue;
                }
            }
            _ => index += 1,
        }
    }

    if brace_depth != 0 {
        return Err("one or more opening braces are not closed".to_owned());
    }
    if let Some(environment) = environments.last() {
        return Err(format!("environment `{environment}` is not closed"));
    }
    if let Some(key) = citations.difference(&bibliography_items).next() {
        return Err(format!("citation `{key}` has no inline bibliography entry"));
    }
    Ok(())
}

fn read_braced_group(bytes: &[u8], start: usize) -> Option<(String, usize)> {
    let mut index = start;
    while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
        index += 1;
    }
    if bytes.get(index) != Some(&b'{') {
        return None;
    }

    let content_start = index + 1;
    index = content_start;
    let mut depth = 1usize;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index = (index + 2).min(bytes.len()),
            b'{' => {
                depth += 1;
                index += 1;
            }
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    let content = std::str::from_utf8(&bytes[content_start..index]).ok()?;
                    return Some((content.to_owned(), index + 1));
                }
                index += 1;
            }
            _ => index += 1,
        }
    }
    None
}

fn is_citation_command(command: &str) -> bool {
    matches!(
        command,
        "cite"
            | "citep"
            | "citet"
            | "citeauthor"
            | "citeyear"
            | "autocite"
            | "parencite"
            | "textcite"
    )
}

fn is_external_file_command(command: &str) -> bool {
    matches!(
        command,
        "input"
            | "include"
            | "includegraphics"
            | "bibliography"
            | "bibliographystyle"
            | "addbibresource"
            | "includepdf"
            | "lstinputlisting"
            | "inputminted"
            | "verbatiminput"
            | "graphicspath"
            | "import"
            | "subimport"
            | "openin"
            | "openout"
            | "read"
            | "write"
            | "IfFileExists"
            | "InputIfFileExists"
            | "includeonly"
    )
}

fn is_verbatim_environment(environment: &str) -> bool {
    matches!(
        environment,
        "verbatim" | "verbatim*" | "lstlisting" | "minted"
    )
}
