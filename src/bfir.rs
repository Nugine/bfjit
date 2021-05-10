use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BfIR {
    AddVal(u8),  // +
    SubVal(u8),  // -
    AddPtr(u32), // >
    SubPtr(u32), // <
    GetByte,     // ,
    PutByte,     // .
    Jz(u32),     // [
    Jnz(u32),    // ]
}

#[derive(Debug, thiserror::Error)]
pub enum CompileErrorKind {
    #[error("Unclosed left bracket")]
    UnclosedLeftBracket,
    #[error("Unexpected right bracket")]
    UnexpectedRightBracket,
}

#[derive(Debug)]
pub struct CompileError {
    line: u32,
    col: u32,
    kind: CompileErrorKind,
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} at line {}:{}", self.kind, self.line, self.col)
    }
}

impl std::error::Error for CompileError {}

pub fn compile(src: &str) -> Result<Vec<BfIR>, CompileError> {
    let mut code: Vec<BfIR> = vec![];

    let mut stk: Vec<(u32, u32, u32)> = vec![];

    let mut line: u32 = 1;
    let mut col: u32 = 0;

    for ch in src.chars() {
        col += 1;
        match ch {
            '\n' => {
                line += 1;
                col = 0;
            }
            '+' => code.push(BfIR::AddVal(1)),
            '-' => code.push(BfIR::SubVal(1)),
            '>' => code.push(BfIR::AddPtr(1)),
            '<' => code.push(BfIR::SubPtr(1)),
            ',' => code.push(BfIR::GetByte),
            '.' => code.push(BfIR::PutByte),
            '[' => {
                let pos = code.len() as u32;
                stk.push((pos, line, col));
                code.push(BfIR::Jz(u32::max_value()))
            }
            ']' => {
                let right = code.len() as u32;
                let (left, _, _) = stk.pop().ok_or(CompileError {
                    line,
                    col,
                    kind: CompileErrorKind::UnexpectedRightBracket,
                })?;

                code[left as usize] = BfIR::Jz(right);
                code.push(BfIR::Jnz(left))
            }
            _ => {}
        }
    }
    if let Some((_, line, col)) = stk.pop() {
        return Err(CompileError {
            line,
            col,
            kind: CompileErrorKind::UnclosedLeftBracket,
        });
    }
    Ok(code)
}

pub fn optimize(code: &mut Vec<BfIR>) {
    let len = code.len();
    let mut i = 0;
    let mut pc = 0;

    macro_rules! _fold_ir {
        ($variant:ident, $x:ident) => {{
            let mut j = i + 1;
            while j < len {
                if let $variant(d) = code[j] {
                    $x = $x.wrapping_add(d);
                } else {
                    break;
                }
                j += 1;
            }
            i = j;
            code[pc] = $variant($x);
            pc += 1;
        }};
    }

    macro_rules! _normal_ir {
        () => {{
            code[pc] = code[i];
            pc += 1;
            i += 1;
        }};
    }

    use BfIR::*;
    while i < len {
        match code[i] {
            AddPtr(mut x) => _fold_ir!(AddPtr, x),
            SubPtr(mut x) => _fold_ir!(SubPtr, x),
            AddVal(mut x) => _fold_ir!(AddVal, x),
            SubVal(mut x) => _fold_ir!(SubVal, x),
            GetByte => _normal_ir!(),
            PutByte => _normal_ir!(),
            Jz(_) => _normal_ir!(),
            Jnz(_) => _normal_ir!(),
        }
    }

    code.truncate(pc);
    code.shrink_to_fit();
}

#[cfg(test)]
#[test]
fn test_compile() {
    assert_eq!(
        compile("+[,.]").unwrap(),
        vec![
            BfIR::AddVal(1),
            BfIR::Jz(4),
            BfIR::GetByte,
            BfIR::PutByte,
            BfIR::Jnz(1),
        ]
    );

    match compile("[").unwrap_err().kind {
        CompileErrorKind::UnclosedLeftBracket => {}
        _ => panic!(),
    };

    match compile("]").unwrap_err().kind {
        CompileErrorKind::UnexpectedRightBracket => {}
        _ => panic!(),
    };

    let mut code = compile("[+++++]").unwrap();
    optimize(&mut code);
    assert_eq!(code, vec![BfIR::Jz(6), BfIR::AddVal(5), BfIR::Jnz(0)]);
}
