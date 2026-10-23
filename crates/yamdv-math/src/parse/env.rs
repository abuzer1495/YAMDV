//! Environments (R-MATH-02): matrices, cases, aligned/gathered families, `array{…}`, `equation`.

use super::{Env, MathError, Node, Parser, Stop, Tok, err};

impl Parser {
    pub(super) fn env(&mut self, pos: usize) -> Result<Node, MathError> {
        let name = self.raw_arg()?;
        let env = match name.as_str() {
            "matrix" => Env::Matrix,
            "pmatrix" => Env::PMatrix,
            "bmatrix" => Env::BMatrix,
            "Bmatrix" => Env::BraceMatrix,
            "vmatrix" => Env::VMatrix,
            "Vmatrix" => Env::DoubleVMatrix,
            "smallmatrix" => Env::Small,
            "cases" => Env::Cases,
            "dcases" => Env::DCases,
            "aligned" | "align" | "align*" | "split" | "alignat" | "alignat*" | "alignedat" => {
                Env::Aligned
            }
            "gathered" | "gather" | "gather*" | "equation" | "equation*" => Env::Gathered,
            "array" | "darray" => Env::Array,
            _ => return err(format!("unknown environment {name}"), pos),
        };
        if name.starts_with("alignat") || name == "alignedat" {
            // The column-pair count argument carries no layout information here.
            self.raw_arg()?;
        }
        let cols: Vec<char> = if env == Env::Array {
            let spec = self.raw_arg()?;
            let c: Vec<char> = spec
                .chars()
                .filter(|c| matches!(c, 'l' | 'c' | 'r' | '|'))
                .collect();
            if !c.iter().any(|c| *c != '|') {
                return err("array needs a column spec such as {cc}", pos);
            }
            c
        } else {
            Vec::new()
        };
        let mut rows: Vec<Vec<Node>> = vec![Vec::new()];
        loop {
            let cell = self.list(Stop::Cell)?;
            self.count()?;
            if let Some(r) = rows.last_mut() {
                r.push(Node::Group(cell));
            }
            let p = self.pos();
            match self.bump() {
                Some(Tok::Amp) => {}
                Some(Tok::Cmd(c)) if c == "\\" => {
                    // `\\[2pt]`: the optional row gap is accepted and ignored.
                    if self.peek() == Some(&Tok::Char('[')) {
                        while let Some(t) = self.bump() {
                            if t == Tok::Char(']') {
                                break;
                            }
                        }
                    }
                    rows.push(Vec::new());
                }
                Some(Tok::Cmd(c)) if c == "end" => {
                    let end = self.raw_arg()?;
                    if end != name {
                        return err(
                            format!("\\end{{{end}}} does not match \\begin{{{name}}}"),
                            p,
                        );
                    }
                    break;
                }
                _ => return err(format!("missing \\end{{{name}}}"), p),
            }
        }
        // A trailing `\\` leaves an empty last row.
        if rows.len() > 1
            && rows.last().is_some_and(|r| {
                r.len() == 1 && matches!(r.first(), Some(Node::Group(g)) if g.is_empty())
            })
        {
            rows.pop();
        }
        Ok(Node::Matrix { env, rows, cols })
    }
}
