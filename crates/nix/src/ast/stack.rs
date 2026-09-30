use crate::lex::{Quote, QuoteId, Span, Token};

use super::{Call, Cx, Expr, Operand, Operation};

#[derive(Default)]
pub(super) struct Stack<'a> {
    pending: Vec<PendingOperation<'a>>,
}

impl<'a> Stack<'a> {
    pub(super) fn end(&self) -> usize {
        self.pending.last().map_or(0, PendingOperation::end)
    }

    pub(super) fn push_unary(&mut self, operator: Quote) {
        self.pending.push(PendingOperation::Unary(operator));
    }

    /// Returns the completed expression, or queues the next operation.
    pub(super) fn reduce(&mut self, mut expr: Expr<'a>, cx: &mut Cx<'_>) -> Option<Expr<'a>> {
        let next = Continuation::next(cx);

        while let Some(pending) = self.pending.pop() {
            if next
                .as_ref()
                .is_some_and(|next| pending.precedence() < next.precedence())
            {
                self.pending.push(pending);
                break;
            }

            expr = pending.reduce(expr);
        }

        match next {
            Some(next) => {
                self.pending.push(next.into_pending(expr));
                None
            }
            None => Some(expr),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Precedence {
    Additive,
    Multiplicative,
    Unary,
    Call,
}

impl Precedence {
    const fn binary(operator: QuoteId) -> Self {
        match operator {
            QuoteId::Add | QuoteId::Sub => Self::Additive,
            QuoteId::Mul => Self::Multiplicative,
            _ => panic!("expected a binary operator"),
        }
    }
}

enum Continuation {
    Binary(Quote),
    Call,
}

impl Continuation {
    fn next(cx: &mut Cx<'_>) -> Option<Self> {
        cx.next_if(|tk| tk.as_quote().is_some_and(|quote| quote.value.is_op()))
            .and_then(Token::into_quote)
            .map(Self::Binary)
            .or_else(|| cx.peek().is_some_and(starts_argument).then_some(Self::Call))
    }

    const fn precedence(&self) -> Precedence {
        match self {
            Self::Binary(operator) => Precedence::binary(operator.value),
            Self::Call => Precedence::Call,
        }
    }

    const fn into_pending(self, lhs: Expr<'_>) -> PendingOperation<'_> {
        match self {
            Self::Binary(operator) => PendingOperation::Binary(lhs, operator),
            Self::Call => PendingOperation::Call(lhs),
        }
    }
}

enum PendingOperation<'a> {
    Unary(Quote),
    Binary(Expr<'a>, Quote),
    Call(Expr<'a>),
}

impl<'a> PendingOperation<'a> {
    const fn precedence(&self) -> Precedence {
        match self {
            Self::Unary(_) => Precedence::Unary,
            Self::Binary(_, operator) => Precedence::binary(operator.value),
            Self::Call(_) => Precedence::Call,
        }
    }

    fn end(&self) -> usize {
        match self {
            Self::Unary(operator) | Self::Binary(_, operator) => operator.range.end,
            Self::Call(callee) => callee.range().end,
        }
    }

    fn reduce(self, rhs: Expr<'a>) -> Expr<'a> {
        let end = rhs.range().end;
        match self {
            Self::Unary(operator) => Expr::Operation(Operation {
                range: operator.range.start..end,
                operator,
                operand: Box::new(Operand::Unary(rhs)),
            }),
            Self::Binary(lhs, operator) => Expr::Operation(Operation {
                range: lhs.range().start..end,
                operator,
                operand: Box::new(Operand::Binary(lhs, rhs)),
            }),
            Self::Call(callee) => Expr::Call(Call {
                range: callee.range().start..end,
                callee: Box::new(callee),
                argument: Box::new(rhs),
            }),
        }
    }
}

const fn starts_argument(tk: &Token<'_>) -> bool {
    matches!(
        tk,
        Token::Literal(_)
            | Token::Ident(_)
            | Token::Quote(Quote {
                value: QuoteId::SubstL | QuoteId::ParenL | QuoteId::BraceL | QuoteId::BracketL,
                ..
            })
    )
}
