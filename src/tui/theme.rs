use ratatui::style::{Color, Style};

pub struct Theme;

impl Theme {
    pub fn sand() -> Color {
        Color::Rgb(194, 178, 128)
    }

    pub fn cream() -> Color {
        Color::Rgb(245, 235, 224)
    }

    pub fn dark_brown() -> Color {
        Color::Rgb(87, 66, 46)
    }

    pub fn earth() -> Color {
        Color::Rgb(139, 119, 89)
    }

    pub fn bg() -> Color {
        Color::Rgb(30, 28, 26)
    }

    pub fn accent() -> Color {
        Color::Rgb(180, 120, 60)
    }

    pub fn title_style() -> Style {
        Style::default().fg(Self::sand()).bg(Self::bg())
    }

    pub fn text_style() -> Style {
        Style::default().fg(Self::cream()).bg(Self::bg())
    }

    pub fn accent_style() -> Style {
        Style::default().fg(Self::accent()).bg(Self::bg())
    }

    pub fn dim_style() -> Style {
        Style::default().fg(Self::earth()).bg(Self::bg())
    }

    pub fn highlight_style() -> Style {
        Style::default().fg(Self::sand()).bg(Self::dark_brown())
    }
}
