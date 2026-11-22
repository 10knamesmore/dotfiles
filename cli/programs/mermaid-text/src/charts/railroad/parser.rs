use super::{Atom, Dialect, Expr, Grammar};

pub(super) struct Parser<'a> {
    source: &'a str,
    rest: &'a str,
    dialect: Dialect,
}

impl<'a> Parser<'a> {
    pub(super) fn new(source: &'a str, dialect: Dialect) -> Self {
        Self {
            source,
            rest: source,
            dialect,
        }
    }

    fn error(&self, message: &str) -> String {
        self.error_at(self.rest, message)
    }

    fn error_at(&self, rest: &str, message: &str) -> String {
        let before = &self.source[..self.source.len() - rest.len()];
        let line = 2 + before.bytes().filter(|b| *b == b'\n').count();
        let col = 1 + before.rsplit('\n').next().unwrap().chars().count();
        let found: String = rest.chars().take_while(|c| *c != '\n').take(20).collect();
        format!(
            "railroad line {line}, column {col}: {message} ({})",
            if rest.is_empty() {
                "end of input".into()
            } else {
                format!("at {found:?}")
            }
        )
    }

    fn line(&mut self) -> &'a str {
        let (line, rest) = self.rest.split_once('\n').unwrap_or((self.rest, ""));
        self.rest = rest;
        line
    }

    fn skip(&mut self) -> Result<(), String> {
        loop {
            self.rest = self.rest.trim_start_matches([' ', '\t', '\n', '\r']);
            if self.rest.starts_with("%%")
                || (self.dialect == Dialect::Peg && self.rest.starts_with('#'))
            {
                self.line();
                continue;
            }
            let end = match self.dialect {
                Dialect::Ir | Dialect::Ebnf if self.rest.starts_with("/*") => "*/",
                Dialect::Ebnf if self.rest.starts_with("(*") => "*)",
                _ => break,
            };
            self.rest = self.rest[2..]
                .split_once(end)
                .ok_or_else(|| self.error(&format!("unclosed comment; expected {end}")))?
                .1;
        }
        Ok(())
    }

    fn eat(&mut self, token: &str) -> bool {
        if let Some(rest) = self.rest.strip_prefix(token) {
            self.rest = rest;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, token: &str) -> Result<(), String> {
        self.skip()?;
        if self.eat(token) {
            Ok(())
        } else {
            Err(self.error(&format!("expected `{token}`")))
        }
    }

    fn id_start(&self, c: char) -> bool {
        c.is_ascii_alphabetic() || (self.dialect != Dialect::Abnf && c == '_')
    }

    fn name(&mut self) -> Result<&'a str, String> {
        if !self.rest.starts_with(|c| self.id_start(c)) {
            return Err(self.error("expected an ASCII rule or constructor name"));
        }
        let end = self
            .rest
            .find(|c: char| !(self.id_start(c) || c.is_ascii_digit() || c == '-'))
            .unwrap_or(self.rest.len());
        let (name, rest) = self.rest.split_at(end);
        self.rest = rest;
        Ok(name)
    }

    fn string(&mut self) -> Result<&'a str, String> {
        let Some(quote @ ('"' | '\'')) = self.rest.chars().next() else {
            return Err(self.error("expected a quoted string"));
        };
        if self.dialect == Dialect::Abnf && quote != '"' {
            return Err(self.error("ABNF literals require double quotes"));
        }
        let mut chars = self.rest.char_indices().skip(1);
        while let Some((i, c)) = chars.next() {
            if c == quote {
                let (s, rest) = self.rest.split_at(i + 1);
                self.rest = rest;
                return Ok(s);
            }
            if self.dialect != Dialect::Abnf && c == '\\' {
                chars.next();
            }
        }
        Err(self.error("unclosed quoted string"))
    }

    pub(super) fn diagram(mut self) -> Result<Grammar<'a>, String> {
        let mut title = None;
        let mut rules = Vec::new();
        loop {
            self.skip()?;
            if self.rest.is_empty() {
                break;
            }
            if self.dialect == Dialect::Abnf && self.rest.starts_with(';') {
                self.line();
                continue;
            }
            let name = self.name()?;
            if rules.is_empty() {
                match name {
                    "title" if self.rest.starts_with([' ', '\t']) => {
                        let text = self.line().split("%%").next().unwrap().trim();
                        let text = text
                            .strip_prefix('"')
                            .and_then(|s| s.strip_suffix('"'))
                            .or_else(|| text.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
                            .unwrap_or(text);
                        title = Some(printable(text));
                        continue;
                    }
                    "accTitle" | "accDescr" => {
                        self.skip()?;
                        if self.eat(":") {
                            self.line();
                            continue;
                        }
                        if name == "accDescr" && self.eat("{") {
                            self.rest = self
                                .rest
                                .split_once('}')
                                .ok_or_else(|| self.error("unclosed accDescr block"))?
                                .1;
                            continue;
                        }
                    }
                    _ => {}
                }
            }
            self.skip()?;
            match self.dialect {
                Dialect::Ebnf if self.rest.starts_with("::=") => self.expect("::=")?,
                Dialect::Peg => self.expect("<-")?,
                Dialect::Abnf if self.rest.starts_with("=/") => {
                    return Err(self.error("incremental ABNF alternatives `=/` are unsupported"));
                }
                _ => self.expect("=")?,
            }
            let expr = if self.dialect == Dialect::Ir {
                self.ir(0)?
            } else {
                self.choice(0)?
            };
            self.expect(";")?;
            rules.push((name, expr));
            if self.dialect == Dialect::Abnf {
                // A terminator can also introduce an end-of-line ABNF comment.
                let tail = self.rest.trim_start_matches([' ', '\t']);
                let after_name =
                    tail.trim_start_matches(|c: char| c.is_ascii_alphanumeric() || c == '-');
                let next_rule = tail.starts_with(|c: char| c.is_ascii_alphabetic())
                    && after_name.trim_start_matches([' ', '\t']).starts_with('=');
                if !tail.is_empty() && !tail.starts_with('\n') && !next_rule {
                    self.line();
                }
            }
        }
        if rules.is_empty() {
            return Err(self.error("expected at least one grammar rule"));
        }
        Ok(Grammar {
            title: title.filter(|text| !text.is_empty()),
            rules,
        })
    }

    fn depth(&self, depth: usize) -> Result<(), String> {
        if depth > 40 {
            Err(self.error("expression nesting exceeds 40 levels"))
        } else {
            Ok(())
        }
    }

    fn ir(&mut self, depth: usize) -> Result<Expr, String> {
        self.depth(depth)?;
        self.skip()?;
        let start = self.rest;
        let name = self.name()?;
        self.expect("(")?;
        self.skip()?;
        let expr = match name {
            "terminal" | "nonterminal" | "special" => {
                let s = self.string()?;
                let inner = printable(&s[1..s.len() - 1]);
                match name {
                    "nonterminal" => Expr::Atom(inner, Atom::Nonterminal),
                    "special" => Expr::Atom(inner, Atom::Special),
                    _ => Expr::Atom(printable(s), Atom::Terminal),
                }
            }
            "sequence" | "choice" => {
                let mut items = vec![self.ir(depth + 1)?];
                self.skip()?;
                while self.eat(",") {
                    items.push(self.ir(depth + 1)?);
                    self.skip()?;
                }
                if name == "sequence" {
                    Expr::Sequence(items)
                } else {
                    Expr::Choice(items, false)
                }
            }
            "optional" => self.ir(depth + 1)?.repeat(0, Some(1)),
            "zeroOrMore" => self.ir(depth + 1)?.repeat(0, None),
            "oneOrMore" => self.ir(depth + 1)?.repeat(1, None),
            _ => return Err(self.error_at(start, &format!("unknown IR constructor `{name}`"))),
        };
        self.expect(")")?;
        Ok(expr)
    }

    fn choice(&mut self, depth: usize) -> Result<Expr, String> {
        self.depth(depth)?;
        let sep = if self.dialect == Dialect::Ebnf {
            "|"
        } else {
            "/"
        };
        let mut alternatives = Vec::new();
        loop {
            let mut items = Vec::new();
            loop {
                items.push(self.term(depth)?);
                self.skip()?;
                if self.dialect == Dialect::Ebnf && self.eat(",") {
                    continue;
                }
                if self.rest.is_empty()
                    || self.rest.starts_with([';', ')', ']', '}'])
                    || self.rest.starts_with(sep)
                {
                    break;
                }
            }
            alternatives.push(Expr::Sequence(items));
            if !self.eat(sep) {
                break;
            }
        }
        Ok(Expr::Choice(alternatives, self.dialect == Dialect::Peg))
    }

    fn term(&mut self, depth: usize) -> Result<Expr, String> {
        self.skip()?;
        if self.dialect == Dialect::Abnf {
            let bounds = self.bounds()?;
            let atom = self.primary(depth)?;
            return Ok(if let Some((min, max)) = bounds {
                atom.repeat(min, max)
            } else {
                atom
            });
        }
        let predicate = if self.dialect == Dialect::Peg && self.eat("&") {
            Some("lookahead")
        } else if self.dialect == Dialect::Peg && self.eat("!") {
            Some("negative lookahead")
        } else {
            None
        };
        let mut expr = self.primary(depth)?;
        let mut postfixes = 0;
        loop {
            self.skip()?;
            expr = if self.eat("*") {
                expr.repeat(0, None)
            } else if self.eat("+") {
                expr.repeat(1, None)
            } else if self.special().is_none() && self.eat("?") {
                expr.repeat(0, Some(1))
            } else if self.dialect == Dialect::Ebnf && self.eat("-") {
                expr.scope("match", Some(self.primary(depth)?))
            } else {
                break;
            };
            postfixes += 1;
            if self.dialect == Dialect::Peg {
                break;
            }
            if postfixes > 16 {
                return Err(self.error("more than 16 consecutive EBNF postfix operators"));
            }
        }
        Ok(if let Some(test) = predicate {
            expr.scope(test, None)
        } else {
            expr
        })
    }

    fn special(&self) -> Option<&'a str> {
        if self.dialect != Dialect::Ebnf {
            return None;
        }
        let rest = self.rest.strip_prefix('?')?;
        let end = rest.find([';', '?'])?;
        (rest.as_bytes()[end] == b'?' && !rest[..end].trim().is_empty()).then_some(&rest[..end])
    }

    fn primary(&mut self, depth: usize) -> Result<Expr, String> {
        self.skip()?;
        if self.rest.starts_with(['"', '\'']) {
            return Ok(Expr::Atom(printable(self.string()?), Atom::Terminal));
        }
        if self.rest.starts_with(|c| self.id_start(c)) {
            return Ok(Expr::Atom(self.name()?.into(), Atom::Nonterminal));
        }
        if self.eat("(") {
            let expr = self.choice(depth + 1)?;
            self.expect(")")?;
            return Ok(expr);
        }
        if self.dialect != Dialect::Peg && self.eat("[") {
            let expr = self.choice(depth + 1)?;
            self.expect("]")?;
            return Ok(expr.repeat(0, Some(1)));
        }
        if self.dialect == Dialect::Ebnf && self.eat("{") {
            let expr = self.choice(depth + 1)?;
            self.expect("}")?;
            return Ok(expr.repeat(0, None));
        }
        if let Some(s) = self.special() {
            self.rest = &self.rest[s.len() + 2..];
            return Ok(Expr::Atom(printable(s.trim()), Atom::Special));
        }
        if self.dialect == Dialect::Abnf && self.rest.starts_with('%') {
            return self.numeric();
        }
        if self.dialect == Dialect::Peg && self.eat(".") {
            return Ok(Expr::Atom("<any>".into(), Atom::Any));
        }
        let message = match (self.dialect, self.rest.chars().next()) {
            (Dialect::Peg, Some('[')) => "PEG character classes are unsupported",
            (Dialect::Peg, Some('{')) => "PEG semantic actions are unsupported",
            (Dialect::Ebnf, Some('?')) => "expected a nonempty EBNF special sequence `? text ?`",
            (Dialect::Ebnf, Some('0'..='9')) => "counted EBNF repetition is unsupported",
            (Dialect::Abnf, Some('<')) => "ABNF prose values are unsupported",
            (_, Some('=')) => "unexpected assignment; the previous rule may be missing `;`",
            _ => "expected a grammar element",
        };
        Err(self.error(message))
    }

    fn decimal(&mut self) -> Result<Option<usize>, String> {
        let len = self.rest.bytes().take_while(u8::is_ascii_digit).count();
        if len == 0 {
            return Ok(None);
        }
        let value = self.rest[..len]
            .parse()
            .map_err(|_| self.error("repetition count is too large"))?;
        self.rest = &self.rest[len..];
        Ok(Some(value))
    }

    fn bounds(&mut self) -> Result<Option<(usize, Option<usize>)>, String> {
        let start = self.rest;
        let lower = self.decimal()?;
        if !self.eat("*") {
            return Ok(lower.map(|n| (n, Some(n))));
        }
        let upper = self.decimal()?;
        let min = lower.unwrap_or(0);
        if upper.is_some_and(|n| n < min) {
            return Err(self.error_at(start, "repetition maximum is below the minimum"));
        }
        Ok(Some((min, upper)))
    }

    fn numeric(&mut self) -> Result<Expr, String> {
        let base = match self.rest.as_bytes().get(1).map(u8::to_ascii_lowercase) {
            Some(b'x') => 16,
            Some(b'd') => 10,
            Some(b'b') => 2,
            Some(b'i' | b's') => {
                return Err(self.error("ABNF %i and %s string modifiers are unsupported"));
            }
            _ => {
                return Err(self.error("expected an ABNF numeric value starting with %x, %d or %b"));
            }
        };
        let start = self.rest;
        self.rest = &self.rest[2..];
        loop {
            let n = self.rest.bytes().take_while(u8::is_ascii_hexdigit).count();
            if n == 0 || !self.rest[..n].chars().all(|c| c.is_digit(base)) {
                return Err(self.error("invalid digit in ABNF numeric value"));
            }
            self.rest = &self.rest[n..];
            if !self.eat(".") && !self.eat("-") {
                break;
            }
        }
        Ok(Expr::Atom(
            start[..start.len() - self.rest.len()].into(),
            Atom::Numeric,
        ))
    }
}

fn printable(text: &str) -> String {
    text.replace('\t', "\\t")
        .replace('\r', "\\r")
        .replace('\n', "\\n")
}
