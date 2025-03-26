use std::{cell::Cell, fmt};

use super::Statement;

/// Displays an item surrounded by brackets, e.g. `[foo]`
pub struct DisplayBracketed<T>(pub T);
impl<T: fmt::Display> fmt::Display for DisplayBracketed<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{}]", &self.0)
    }
}

/// Displays a slice of items surrounded by brackets, e.g. `[foo, bar]`
pub struct DisplayList<'a, T>(pub &'a [T]);
impl<T: fmt::Display> fmt::Display for DisplayList<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[{}]", DisplayCsv::new(self.0.iter()))
    }
}

/// Displays an item surrounded by parentheses, e.g. `(foo)`
pub struct DisplayParenthesized<T>(pub T);
impl<T: fmt::Display> fmt::Display for DisplayParenthesized<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({})", &self.0)
    }
}

/// Displays a slice of items surrounded by parentheses, e.g. `(foo, bar)`
pub struct DisplayTuple<'a, T>(pub &'a [T]);
impl<T: fmt::Display> fmt::Display for DisplayTuple<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({})", DisplayCsv::new(self.0.iter()))
    }
}

/// Displays a slice of items with their types surrounded by parentheses,
/// e.g. `(foo: felt, bar: felt[12])`
pub struct DisplayTypedTuple<'a, V, T>(pub &'a [(V, T)]);
impl<V: fmt::Display, T: fmt::Display> fmt::Display for DisplayTypedTuple<'_, V, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "({})",
            DisplayCsv::new(self.0.iter().map(|(v, t)| format!("{}: {}", v, t)))
        )
    }
}

/// Displays one or more items separated by commas, e.g. `foo, bar`
pub struct DisplayCsv<T>(Cell<Option<T>>);
impl<T, I> DisplayCsv<I>
where
    T: fmt::Display,
    I: Iterator<Item = T>,
{
    pub fn new(iter: I) -> Self {
        Self(Cell::new(Some(iter)))
    }
}
impl<T, I> fmt::Display for DisplayCsv<I>
where
    T: fmt::Display,
    I: Iterator<Item = T>,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let iter = self.0.take().unwrap();
        for (i, item) in iter.enumerate() {
            if i > 0 {
                f.write_str(", ")?;
            }
            write!(f, "{}", item)?;
        }
        Ok(())
    }
}

pub struct DisplayStatement<'a> {
    pub statement: &'a Statement,
    pub indent: usize,
}
impl DisplayStatement<'_> {
    const INDENT: &'static str = "    ";

    fn write_indent(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for _ in 0..self.indent {
            f.write_str(Self::INDENT)?;
        }
        Ok(())
    }
}
impl fmt::Display for DisplayStatement<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.write_indent(f)?;
        match self.statement {
            Statement::Let(ref expr) => {
                let display = DisplayLet {
                    let_expr: expr,
                    indent: self.indent,
                    in_expr_position: false,
                };
                write!(f, "{display}")
            }
            Statement::Enforce(ref expr) => {
                write!(f, "enf {}", expr)
            }
            Statement::EnforceIf(ref expr, ref selector) => {
                write!(f, "enf {} when {}", expr, selector)
            }
            Statement::EnforceAll(ref expr) => {
                write!(f, "enf {}", expr)
            }
            Statement::Expr(ref expr) => write!(f, "return {}", expr),
            Statement::BusEnforce(ref expr) => write!(f, "enf {}", expr),
        }
    }
}

pub struct DisplayLet<'a> {
    pub let_expr: &'a super::Let,
    pub indent: usize,
    pub in_expr_position: bool,
}
impl DisplayLet<'_> {
    const INDENT: &'static str = "    ";

    fn write_indent(&self, f: &mut fmt::Formatter) -> fmt::Result {
        for _ in 0..self.indent {
            f.write_str(Self::INDENT)?;
        }
        Ok(())
    }
}
impl fmt::Display for DisplayLet<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use core::fmt::Write;

        self.write_indent(f)?;
        match &self.let_expr.value {
            super::Expr::Let(value) => {
                writeln!(f, "let {} = {{", self.let_expr.name)?;
                let display = DisplayLet {
                    let_expr: value,
                    indent: self.indent + 1,
                    in_expr_position: true,
                };
                writeln!(f, "{display}")?;
                self.write_indent(f)?;
                if self.in_expr_position {
                    f.write_str("} in {\n")?;
                } else {
                    f.write_str("}\n")?;
                }
            }
            value => {
                write!(f, "let {} = {}", self.let_expr.name, value)?;
                if self.in_expr_position {
                    f.write_str(" in {\n")?;
                } else {
                    f.write_char('\n')?;
                }
            }
        }
        for stmt in self.let_expr.body.iter() {
            writeln!(f, "{}", stmt.display(self.indent + 1))?;
        }
        if self.in_expr_position {
            self.write_indent(f)?;
            f.write_char('}')?;
        }
        Ok(())
    }
}
