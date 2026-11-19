//! Lexer and parser for the TypeScript subset the Fluent UI token files use: `const` declarations of
//! object, array, string, number and reference expressions. Anything else is an error.

pub(crate) struct ParseError {
    pub(crate) line: usize,
    pub(crate) message: String,
}

fn err(line: usize, message: impl Into<String>) -> ParseError {
    ParseError {
        line,
        message: message.into(),
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Part {
    Text(String),
    Var(String),
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Tok {
    Ident(String),
    Num(String),
    Str(Vec<Part>),
    Punct(char),
}

pub(crate) struct Token {
    pub(crate) tok: Tok,
    pub(crate) line: usize,
}

#[derive(Debug)]
pub(crate) enum Expr {
    Str(String),
    Num(String),
    Ident(String),
    Member(Box<Expr>, String),
    Index(Box<Expr>, Box<Expr>),
    Object(Vec<(String, Expr)>),
    Array(Vec<Expr>),
}

pub(crate) struct Const {
    pub(crate) name: String,
    pub(crate) value: Expr,
}

struct Lexer {
    chars: Vec<char>,
    i: usize,
    line: usize,
}

impl Lexer {
    fn at(&self, offset: usize) -> Option<char> {
        self.chars.get(self.i + offset).copied()
    }

    fn take_while(&mut self, keep: impl Fn(char, Option<char>) -> bool) -> String {
        let mut text = String::new();
        while let Some(c) = self.at(0).filter(|&c| keep(c, self.at(1))) {
            text.push(c);
            self.i += 1;
        }
        text
    }

    fn block_comment(&mut self) -> Result<(), ParseError> {
        self.i += 2;
        loop {
            match (self.at(0), self.at(1)) {
                (Some('*'), Some('/')) => {
                    self.i += 2;
                    return Ok(());
                }
                (Some('\n'), _) => self.line += 1,
                (None, _) => return Err(err(self.line, "unterminated comment")),
                _ => {}
            }
            self.i += 1;
        }
    }

    fn quoted(&mut self, quote: char) -> Result<Vec<Part>, ParseError> {
        let start = self.line;
        self.i += 1;
        let text = self.take_while(|c, _| c != quote && c != '\\' && c != '\n');
        if self.at(0) != Some(quote) {
            return Err(err(start, "unsupported string literal"));
        }
        self.i += 1;
        Ok(vec![Part::Text(text)])
    }

    fn template(&mut self) -> Result<Vec<Part>, ParseError> {
        let start = self.line;
        let mut parts = Vec::new();
        let mut text = String::new();
        self.i += 1;
        loop {
            match self.at(0) {
                Some('`') => break,
                Some('\\') | None => return Err(err(start, "unsupported template literal")),
                Some('$') if self.at(1) == Some('{') => {
                    if !text.is_empty() {
                        parts.push(Part::Text(std::mem::take(&mut text)));
                    }
                    self.i += 2;
                    let name = self.take_while(|c, _| c.is_ascii_alphanumeric() || c == '_');
                    if self.at(0) != Some('}') {
                        return Err(err(self.line, "template interpolation must be an identifier"));
                    }
                    parts.push(Part::Var(name));
                }
                Some(c) => {
                    self.line += usize::from(c == '\n');
                    text.push(c);
                }
            }
            self.i += 1;
        }
        self.i += 1;
        if !text.is_empty() || parts.is_empty() {
            parts.push(Part::Text(text));
        }
        Ok(parts)
    }
}

pub(crate) fn lex(src: &str) -> Result<Vec<Token>, ParseError> {
    let mut lx = Lexer {
        chars: src.chars().collect(),
        i: 0,
        line: 1,
    };
    let mut out = Vec::new();
    while let Some(c) = lx.at(0) {
        let line = lx.line;
        let tok = match c {
            '\n' => {
                lx.line += 1;
                lx.i += 1;
                continue;
            }
            c if c.is_whitespace() => {
                lx.i += 1;
                continue;
            }
            '/' if lx.at(1) == Some('/') => {
                lx.take_while(|c, _| c != '\n');
                continue;
            }
            '/' if lx.at(1) == Some('*') => {
                lx.block_comment()?;
                continue;
            }
            '\'' | '"' => Tok::Str(lx.quoted(c)?),
            '`' => Tok::Str(lx.template()?),
            c if c.is_ascii_digit() => Tok::Num(
                lx.take_while(|c, next| c.is_ascii_digit() || (c == '.' && next.is_some_and(|n| n.is_ascii_digit()))),
            ),
            c if c.is_ascii_alphabetic() || c == '_' || c == '$' => {
                Tok::Ident(lx.take_while(|c, _| c.is_ascii_alphanumeric() || c == '_' || c == '$'))
            }
            '{' | '}' | '[' | ']' | '(' | ')' | ':' | ';' | ',' | '.' | '=' | '<' | '>' | '?' | '|' | '&' | '+' => {
                lx.i += 1;
                Tok::Punct(c)
            }
            other => return Err(err(line, format!("unsupported character {other:?}"))),
        };
        out.push(Token { tok, line });
    }
    Ok(out)
}

pub(crate) struct Parser<'a> {
    toks: &'a [Token],
    pub(crate) pos: usize,
}

impl<'a> Parser<'a> {
    pub(crate) const fn new(toks: &'a [Token], pos: usize) -> Self {
        Self { toks, pos }
    }

    fn line(&self) -> usize {
        self.toks.get(self.pos).or(self.toks.last()).map_or(0, |t| t.line)
    }

    fn fail<T>(&self, message: impl Into<String>) -> Result<T, ParseError> {
        Err(err(self.line(), message))
    }

    fn peek(&self) -> Option<&Tok> {
        self.toks.get(self.pos).map(|t| &t.tok)
    }

    fn eat(&mut self, c: char) -> bool {
        let hit = self.peek() == Some(&Tok::Punct(c));
        self.pos += usize::from(hit);
        hit
    }

    fn expect(&mut self, c: char) -> Result<(), ParseError> {
        if self.eat(c) {
            Ok(())
        } else {
            self.fail(format!("expected {c:?}, found {:?}", self.peek()))
        }
    }

    fn ident(&mut self) -> Result<String, ParseError> {
        match self.peek() {
            Some(Tok::Ident(name)) => {
                let name = name.clone();
                self.pos += 1;
                Ok(name)
            }
            other => self.fail(format!("expected identifier, found {other:?}")),
        }
    }

    pub(crate) fn expr(&mut self) -> Result<Expr, ParseError> {
        let mut e = match self.peek().cloned() {
            Some(Tok::Ident(name)) => {
                self.pos += 1;
                Expr::Ident(name)
            }
            Some(Tok::Num(n)) => {
                self.pos += 1;
                Expr::Num(n)
            }
            Some(Tok::Str(parts)) => {
                self.pos += 1;
                match parts.as_slice() {
                    [Part::Text(text)] => Expr::Str(text.clone()),
                    _ => return self.fail("template literal with interpolation in a value"),
                }
            }
            Some(Tok::Punct('{')) => {
                self.pos += 1;
                self.object()?
            }
            Some(Tok::Punct('[')) => {
                self.pos += 1;
                self.array()?
            }
            other => return self.fail(format!("unsupported expression start {other:?}")),
        };
        loop {
            if self.eat('.') {
                e = Expr::Member(Box::new(e), self.ident()?);
            } else if self.eat('[') {
                let index = self.expr()?;
                self.expect(']')?;
                e = Expr::Index(Box::new(e), Box::new(index));
            } else {
                return Ok(e);
            }
        }
    }

    fn object(&mut self) -> Result<Expr, ParseError> {
        let mut entries = Vec::new();
        while !self.eat('}') {
            let key = match self.peek().cloned() {
                Some(Tok::Ident(name) | Tok::Num(name)) => name,
                Some(Tok::Str(parts)) => match parts.as_slice() {
                    [Part::Text(text)] => text.clone(),
                    _ => return self.fail("interpolated object key"),
                },
                other => return self.fail(format!("unsupported object key {other:?}")),
            };
            self.pos += 1;
            let value = if self.eat(':') {
                self.expr()?
            } else {
                Expr::Ident(key.clone())
            };
            entries.push((key, value));
            if !self.eat(',') && self.peek() != Some(&Tok::Punct('}')) {
                return self.fail("expected ',' or '}'");
            }
        }
        Ok(Expr::Object(entries))
    }

    fn array(&mut self) -> Result<Expr, ParseError> {
        let mut items = Vec::new();
        while !self.eat(']') {
            items.push(self.expr()?);
            if !self.eat(',') && self.peek() != Some(&Tok::Punct(']')) {
                return self.fail("expected ',' or ']'");
            }
        }
        Ok(Expr::Array(items))
    }

    fn skip_to(&mut self, c: char) -> Result<(), ParseError> {
        while !self.eat(c) {
            if self.peek().is_none() {
                return self.fail(format!("missing {c:?}"));
            }
            self.pos += 1;
        }
        Ok(())
    }

    fn skip_parens(&mut self) -> Result<(), ParseError> {
        self.expect('(')?;
        let mut depth = 1;
        while depth > 0 {
            match self.peek() {
                Some(Tok::Punct('(')) => depth += 1,
                Some(Tok::Punct(')')) => depth -= 1,
                None => return self.fail("unbalanced parentheses"),
                _ => {}
            }
            self.pos += 1;
        }
        Ok(())
    }

    fn declaration(&mut self) -> Result<Const, ParseError> {
        let mut word = self.ident()?;
        if word == "export" {
            word = self.ident()?;
        }
        if word != "const" {
            return self.fail(format!("unsupported statement `{word}`"));
        }
        let name = self.ident()?;
        while !self.eat('=') {
            if matches!(self.peek(), Some(Tok::Punct(';' | '{' | '(')) | None) {
                return self.fail("unsupported type annotation");
            }
            self.pos += 1;
        }
        let value = if self.peek() == Some(&Tok::Punct('(')) {
            self.skip_parens()?;
            while !(self.eat('=') && self.eat('>')) {
                if self.peek().is_none() {
                    return self.fail("arrow function without '=>'");
                }
                self.pos += 1;
            }
            self.expect('(')?;
            let value = self.expr()?;
            self.expect(')')?;
            value
        } else {
            self.expr()?
        };
        if self.peek() == Some(&Tok::Ident("as".to_owned())) {
            self.pos += 1;
            if self.ident()? != "const" {
                return self.fail("only `as const` assertions are supported");
            }
        }
        self.expect(';')?;
        Ok(Const { name, value })
    }
}

pub(crate) fn module(toks: &[Token]) -> Result<Vec<Const>, ParseError> {
    let mut p = Parser::new(toks, 0);
    let mut out = Vec::new();
    while p.peek().is_some() {
        if p.peek() == Some(&Tok::Ident("import".to_owned())) {
            p.skip_to(';')?;
        } else {
            out.push(p.declaration()?);
        }
    }
    Ok(out)
}
