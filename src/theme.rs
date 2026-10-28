use amane::Color;

// шрифт из waybar (style.css): JetBrainsMono Nerd Font
pub const FONT: &str = "JetBrainsMono Nerd Font";

// gruvbox: bg/text/active/hover взяты из waybar (colors-gruvbox.css)
pub struct Theme {
    pub bg: Color,
    pub surface: Color,
    pub hover: Color,
    pub border: Color,
    pub text: Color,
    pub muted: Color,
    pub dim: Color,
    pub accent: Color,
    pub on_accent: Color,
    pub green: Color,
    pub yellow: Color,
    pub red: Color,
}

pub fn theme() -> Theme {
    Theme {
        bg: Color::from("#1d2021"),
        surface: Color::from("#282828"),
        hover: Color::from("#3c3836"),
        border: Color::from("#504945"),
        text: Color::from("#ebdbb2"),
        muted: Color::from("#a89984"),
        dim: Color::from("#665c54"),
        accent: Color::from("#d65d0e"),
        on_accent: Color::from("#1d2021"),
        green: Color::from("#b8bb26"),
        yellow: Color::from("#fabd2f"),
        red: Color::from("#fb4934"),
    }
}
