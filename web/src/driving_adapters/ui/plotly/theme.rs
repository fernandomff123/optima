use crate::design_system::tokens;
use plotly::{Layout, common::Font, layout::Axis};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlotlyTheme {
    pub paper_background: &'static str,
    pub plot_background: &'static str,
    pub grid: &'static str,
    pub axis: &'static str,
    pub font: &'static str,
    pub positive: &'static str,
    pub negative: &'static str,
    pub selection: &'static str,
    pub special: &'static str,
    pub hover: &'static str,
}

impl PlotlyTheme {
    pub const fn optima() -> Self {
        Self {
            paper_background: tokens::SURFACE,
            plot_background: tokens::CANVAS,
            grid: tokens::CHART_GRID,
            axis: tokens::TEXT_MUTED_READABLE,
            font: tokens::TEXT_SECONDARY,
            positive: tokens::FINANCE_POSITIVE,
            negative: tokens::FINANCE_NEGATIVE,
            selection: tokens::INTERACTIVE_SOURCE,
            special: tokens::LEVEL_SPECIAL,
            hover: tokens::STATE_HOVER,
        }
    }

    pub fn base_layout(self) -> Layout {
        let axis = || {
            Axis::new()
                .grid_color(self.grid)
                .line_color(self.axis)
                .tick_font(Font::new().color(self.axis))
                .zero_line_color(self.grid)
        };
        Layout::new()
            .paper_background_color(self.paper_background)
            .plot_background_color(self.plot_background)
            .font(Font::new().color(self.font))
            .x_axis(axis())
            .y_axis(axis())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn plotly_theme_reuses_approved_tokens() {
        let theme = PlotlyTheme::optima();
        assert_eq!(theme.paper_background, tokens::SURFACE);
        assert_eq!(theme.plot_background, tokens::CANVAS);
        assert_eq!(theme.grid, tokens::CHART_GRID);
        assert_eq!(theme.positive, tokens::FINANCE_POSITIVE);
        assert_eq!(theme.negative, tokens::FINANCE_NEGATIVE);
        assert_eq!(theme.selection, tokens::INTERACTIVE_SOURCE);
        assert_eq!(theme.special, tokens::LEVEL_SPECIAL);
        assert_eq!(theme.hover, tokens::STATE_HOVER);
        let _ = theme.base_layout();
    }
}
