mod escape;
mod render;
mod validate;

pub use escape::escape_latex;
pub use render::{render_latex, used_safe_fallback};
pub use validate::validate_generated_latex;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LatexTemplate {
    Standard,
    Abnt,
    Ieee,
    Acm,
    Elsevier,
    SpringerNature,
}

impl LatexTemplate {
    pub const EXPORT_OPTIONS: [Self; 5] = [
        Self::Abnt,
        Self::Ieee,
        Self::Acm,
        Self::Elsevier,
        Self::SpringerNature,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            Self::Standard => "Standard LaTeX",
            Self::Abnt => "ABNT · abnTeX2",
            Self::Ieee => "IEEE · IEEEtran",
            Self::Acm => "ACM · acmart",
            Self::Elsevier => "Elsevier · elsarticle",
            Self::SpringerNature => "Springer Nature · sn-jnl",
        }
    }

    pub const fn source_url(self) -> Option<&'static str> {
        match self {
            Self::Standard => None,
            Self::Abnt => Some("https://ctan.org/pkg/abntex2"),
            Self::Ieee => Some("https://ctan.org/pkg/ieeetran"),
            Self::Acm => Some("https://www.acm.org/publications/proceedings-template"),
            Self::Elsevier => Some("https://ctan.org/pkg/elsarticle"),
            Self::SpringerNature => {
                Some("https://www.springernature.com/gp/authors/campaigns/latex-author-support")
            }
        }
    }
}
