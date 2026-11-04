//! Small source-to-source lowering for async functions with sequential,
//! top-level `const`/`let`/`var name = await expression;` statements.
//!
//! This preserves the existing interpreter's synchronous execution model by
//! turning each such await into a Promise continuation. It is intentionally
//! conservative: unsupported await positions are left untouched.

use crate::lexer::{Lexer, Token, TokenKind};

pub(crate) fn lower_async_await(source: &str) -> String {
    let tokens: Vec<Token> = Lexer::new(source).filter_map(Result::ok).collect();
    if tokens.is_empty() {
        return source.to_string();
    }
    let byte_at: Vec<usize> = source
        .char_indices()
        .map(|(byte, _)| byte)
        .chain(std::iter::once(source.len()))
        .collect();
    let mut replacements = Vec::new();
    let mut index = 0;
    while index + 1 < tokens.len() {
        if !matches!(tokens[index].kind, TokenKind::Async)
            || !matches!(tokens[index + 1].kind, TokenKind::Function)
        {
            index += 1;
            continue;
        }
        let Some(open) =
            (index + 2..tokens.len()).find(|&i| matches!(tokens[i].kind, TokenKind::LeftBrace))
        else {
            break;
        };
        let mut depth = 1usize;
        let mut close = open + 1;
        while close < tokens.len() && depth != 0 {
            match tokens[close].kind {
                TokenKind::LeftBrace => depth += 1,
                TokenKind::RightBrace => depth -= 1,
                _ => {}
            }
            close += 1;
        }
        if depth != 0 {
            break;
        }
        let close = close - 1;
        let body_start_char = tokens[open].span.end;
        let body_end_char = tokens[close].span.start;
        let body_start = byte_at[body_start_char];
        let body_end = byte_at[body_end_char];
        let body = &source[body_start..body_end];
        if let Some(lowered) = lower_body(body) {
            replacements.push((body_start, body_end, lowered));
        }
        index = close + 1;
    }
    if replacements.is_empty() {
        return source.to_string();
    }
    let mut output = source.to_string();
    for (start, end, replacement) in replacements.into_iter().rev() {
        output.replace_range(start..end, &replacement);
    }
    output
}

fn lower_body(body: &str) -> Option<String> {
    let tokens: Vec<Token> = Lexer::new(body).filter_map(Result::ok).collect();
    let byte_at: Vec<usize> = body
        .char_indices()
        .map(|(byte, _)| byte)
        .chain(std::iter::once(body.len()))
        .collect();

    // Async network code commonly awaits inside try/catch. Lower the try body
    // to a Promise continuation and attach the catch block to its rejection.
    if let Some(lowered) = lower_try_catch(body, &tokens, &byte_at) {
        return Some(lowered);
    }

    let mut braces = 0usize;
    let mut parens = 0usize;
    let mut brackets = 0usize;
    let mut await_at = None;
    for i in 0..tokens.len() {
        let token = &tokens[i];
        if matches!(token.kind, TokenKind::Await) && braces == 0 && parens == 0 && brackets == 0 {
            await_at = Some(i);
            break;
        }
        match token.kind {
            TokenKind::LeftBrace => braces += 1,
            TokenKind::RightBrace => braces = braces.saturating_sub(1),
            TokenKind::LeftParen => parens += 1,
            TokenKind::RightParen => parens = parens.saturating_sub(1),
            TokenKind::LeftBracket => brackets += 1,
            TokenKind::RightBracket => brackets = brackets.saturating_sub(1),
            _ => {}
        }
    }
    let await_at = await_at?;

    // The supported form is a simple declaration immediately preceding await.
    let start = await_at.checked_sub(3)?;
    let (kind, name) = match (&tokens[start].kind, &tokens[start + 1].kind) {
        (TokenKind::Const, TokenKind::Identifier(name)) => ("const", name.clone()),
        (TokenKind::Let, TokenKind::Identifier(name)) => ("let", name.clone()),
        (TokenKind::Var, TokenKind::Identifier(name)) => ("var", name.clone()),
        _ => return None,
    };
    if !matches!(tokens[start + 2].kind, TokenKind::Eq) {
        return None;
    }

    let mut p = 0usize;
    let mut b = 0usize;
    let mut c = 0usize;
    let mut end = None;
    for i in await_at + 1..tokens.len() {
        match tokens[i].kind {
            TokenKind::LeftParen => p += 1,
            TokenKind::RightParen => p = p.saturating_sub(1),
            TokenKind::LeftBracket => b += 1,
            TokenKind::RightBracket => b = b.saturating_sub(1),
            TokenKind::LeftBrace => c += 1,
            TokenKind::RightBrace => c = c.saturating_sub(1),
            TokenKind::Semicolon if p == 0 && b == 0 && c == 0 => {
                end = Some(i);
                break;
            }
            _ => {}
        }
    }
    let end = end?;
    let decl_start = byte_at[tokens[start].span.start];
    let expr_start = byte_at[tokens[await_at].span.end];
    let expr_end = byte_at[tokens[end].span.start];
    let decl_end = byte_at[tokens[end].span.end];
    let expr = body[expr_start..expr_end].trim();
    if expr.is_empty() {
        return None;
    }
    let prefix = &body[..decl_start];
    let rest = &body[decl_end..];
    let suffix = lower_body(rest).unwrap_or_else(|| rest.to_string());
    let temp = format!("__pixi_await_value_{}", tokens[await_at].span.start);
    Some(format!(
        "{prefix}return Promise.resolve({expr}).then(function({temp}) {{ {kind} {name} = {temp}; {suffix} }});"
    ))
}

fn lower_try_catch(body: &str, tokens: &[Token], byte_at: &[usize]) -> Option<String> {
    let try_at = tokens
        .iter()
        .position(|token| matches!(token.kind, TokenKind::Try))?;
    let try_open =
        (try_at + 1..tokens.len()).find(|&i| matches!(tokens[i].kind, TokenKind::LeftBrace))?;
    let try_close = matching_brace(tokens, try_open)?;
    let catch_at =
        (try_close + 1..tokens.len()).find(|&i| matches!(tokens[i].kind, TokenKind::Catch))?;
    let catch_open =
        (catch_at + 1..tokens.len()).find(|&i| matches!(tokens[i].kind, TokenKind::LeftBrace))?;
    let catch_close = matching_brace(tokens, catch_open)?;
    let try_body = lower_body(
        &body[byte_at[tokens[try_open].span.end]..byte_at[tokens[try_close].span.start]],
    )?;
    let prefix = &body[..byte_at[tokens[try_at].span.start]];
    let catch_body =
        &body[byte_at[tokens[catch_open].span.end]..byte_at[tokens[catch_close].span.start]];
    let catch_name = if matches!(
        tokens.get(catch_at + 1).map(|t| &t.kind),
        Some(TokenKind::LeftParen)
    ) {
        tokens
            .get(catch_at + 2)
            .and_then(|token| match &token.kind {
                TokenKind::Identifier(name) => Some(name.clone()),
                _ => None,
            })
            .unwrap_or_else(|| "error".to_string())
    } else {
        "error".to_string()
    };
    let suffix = &body[byte_at[tokens[catch_close].span.end]..];
    let suffix = lower_body(suffix).unwrap_or_else(|| suffix.to_string());
    Some(format!(
        "{prefix}return Promise.resolve().then(function() {{ {try_body} }}).catch(function({catch_name}) {{ if (typeof __orinium_trace_caught_error === \"function\") __orinium_trace_caught_error({catch_name}, \"{catch_name}\"); {catch_body}}});{suffix}"
    ))
}

fn matching_brace(tokens: &[Token], open: usize) -> Option<usize> {
    let mut depth = 1usize;
    for (index, token) in tokens.iter().enumerate().skip(open + 1) {
        match token.kind {
            TokenKind::LeftBrace => depth += 1,
            TokenKind::RightBrace => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}
