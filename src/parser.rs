use std::iter::Peekable;

#[derive(Debug, Clone)]
pub enum RedirectType {
    CREATE,
    APPEND,
}

#[derive(Debug, Clone)]
pub struct Redirect {
    pub fd: u32,
    pub target: String,
    pub redirect_type: RedirectType,
}

pub fn tokenize(input: &str) -> Result<(Vec<String>, Option<Redirect>), String> {
    let mut current_token = String::new();
    let mut tokens: Vec<String> = Vec::new();
    let mut input_chars = input.chars().peekable();
    let mut is_in_single_quotes = false;
    let mut is_in_double_quotes = false;
    while let Some(ch) = input_chars.next() {
        match ch {
            '\\' if !is_in_single_quotes => {
                handle_escape(&mut current_token, &mut input_chars, is_in_double_quotes)
            }
            '"' => {
                if is_in_single_quotes {
                    current_token.push(ch);
                } else {
                    is_in_double_quotes = !is_in_double_quotes;
                }
            }
            '\'' => {
                if is_in_double_quotes {
                    current_token.push(ch);
                } else {
                    is_in_single_quotes = !is_in_single_quotes;
                }
            }
            ch if ch.is_whitespace() => {
                if is_in_single_quotes || is_in_double_quotes {
                    current_token.push(ch);
                } else if !current_token.is_empty() {
                    tokens.push(current_token.clone());
                    current_token.clear();
                }
            }
            _ => {
                current_token.push(ch);
            }
        }
    }
    if !current_token.is_empty() {
        tokens.push(current_token);
    }

    let redirect = parse_redirect(&mut tokens)?;
    Ok((tokens, redirect))
}

fn handle_escape(
    current_token: &mut String,
    input_chars: &mut Peekable<std::str::Chars<'_>>,
    is_in_double_quotes: bool,
) {
    if let Some(&next_char) = input_chars.peek() {
        if is_in_double_quotes {
            match next_char {
                '"' | '$' | '\\' | '`' | '\n' => {
                    current_token.push(next_char);
                    input_chars.next();
                }
                _ => {
                    current_token.push('\\');
                    current_token.push(next_char);
                    input_chars.next();
                }
            }
        } else {
            current_token.push(next_char);
            input_chars.next();
        }
    } else {
        current_token.push('\\');
    }
}

fn parse_redirect(tokens: &mut Vec<String>) -> Result<Option<Redirect>, String> {
    if tokens.len() < 2 {
        return Ok(None);
    }

    let op_token = tokens[tokens.len() - 2].as_str();
    let split_idx = op_token
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(op_token.len());
    let (fd_part, op_part) = op_token.split_at(split_idx);

    let redirect_type_optional = match op_part {
        ">>" => Some(RedirectType::APPEND),
        ">" => Some(RedirectType::CREATE),
        _ => None,
    };

    let fd_optional = match redirect_type_optional {
        Some(_) => {
            if fd_part.is_empty() {
                Some(1)
            } else {
                Some(
                    fd_part
                        .parse::<u32>()
                        .map_err(|_| format!("invalid file descriptor: {}", fd_part))?,
                )
            }
        }
        None => None,
    };

    if let (Some(fd), Some(redirect_type)) = (fd_optional, redirect_type_optional) {
        let filename = tokens
            .pop()
            .ok_or_else(|| "missing file name for redirect".to_string())?;
        tokens.pop();
        Ok(Some(Redirect {
            fd,
            target: filename,
            redirect_type,
        }))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_basic_command() {
        let (tokens, redirect) = tokenize("echo hello world").unwrap();
        assert_eq!(tokens, vec!["echo", "hello", "world"]);
        assert!(redirect.is_none());
    }

    #[test]
    fn preserves_whitespace_inside_quotes() {
        let (tokens, redirect) = tokenize("echo \"hello world\"").unwrap();
        assert_eq!(tokens, vec!["echo", "hello world"]);
        assert!(redirect.is_none());
    }

    #[test]
    fn extracts_redirect_information() {
        let (tokens, redirect) = tokenize("echo hi > out.txt").unwrap();
        assert_eq!(tokens, vec!["echo", "hi"]);

        let redirect = redirect.expect("expected redirect");
        assert_eq!(redirect.fd, 1);
        assert_eq!(redirect.target, "out.txt");
        assert!(matches!(redirect.redirect_type, RedirectType::CREATE));
    }

    #[test]
    fn handles_escape_sequences() {
        let (tokens, redirect) = tokenize(r"echo foo\ bar").unwrap();
        assert_eq!(tokens, vec!["echo", "foo bar"]);
        assert!(redirect.is_none());
    }
}
