use eframe::egui::{self, Color32, CornerRadius, Frame, Margin, Stroke};

#[derive(Debug, Clone)]
pub struct Theme {
    pub surface_0: Color32,
    pub surface_1: Color32,
    pub surface_2: Color32,
    pub surface_3: Color32,
    pub accent_primary: Color32,
    pub accent_hover: Color32,
    pub accent_muted: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub danger: Color32,
    pub text_primary: Color32,
    pub text_muted: Color32,
    pub border_subtle: Color32,
    pub input_focus_glow: Color32,
    pub top_bar_gradient_end: Color32,
    pub spacing_4: f32,
    pub spacing_8: f32,
    pub spacing_12: f32,
    pub spacing_16: f32,
    pub spacing_24: f32,
    pub spacing_32: f32,
    pub radius_8: u8,
    pub radius_10: u8,
    pub radius_12: u8,
    pub button_height: f32,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            surface_0: Color32::from_rgb(0x0F, 0x11, 0x15),
            surface_1: Color32::from_rgb(0x16, 0x1A, 0x20),
            surface_2: Color32::from_rgb(0x1C, 0x22, 0x2B),
            surface_3: Color32::from_rgb(0x22, 0x2A, 0x35),
            accent_primary: Color32::from_rgb(0x3B, 0x82, 0xF6),
            accent_hover: Color32::from_rgb(0x4C, 0x8E, 0xF7),
            accent_muted: Color32::from_rgb(0x2F, 0x6E, 0xD8),
            success: Color32::from_rgb(0x22, 0xC5, 0x5E),
            warning: Color32::from_rgb(0xF5, 0x9E, 0x0B),
            danger: Color32::from_rgb(0xEF, 0x44, 0x44),
            text_primary: Color32::from_rgb(0xE6, 0xED, 0xF3),
            text_muted: Color32::from_rgb(0x8B, 0x94, 0x9E),
            border_subtle: Color32::from_rgba_premultiplied(255, 255, 255, 13),
            input_focus_glow: Color32::from_rgba_premultiplied(0x3B, 0x82, 0xF6, 51),
            top_bar_gradient_end: Color32::from_rgb(0x14, 0x18, 0x1E),
            spacing_4: 4.0,
            spacing_8: 8.0,
            spacing_12: 12.0,
            spacing_16: 16.0,
            spacing_24: 24.0,
            spacing_32: 32.0,
            radius_8: 8,
            radius_10: 10,
            radius_12: 12,
            button_height: 35.0,
        }
    }
}

impl Theme {
    pub fn apply_visuals(&self, ctx: &egui::Context) {
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = self.surface_1;
        visuals.override_text_color = Some(self.text_primary);
        visuals.widgets.noninteractive.fg_stroke.color = self.text_primary;
        visuals.widgets.noninteractive.bg_fill = self.surface_2;
        visuals.widgets.noninteractive.weak_bg_fill = self.surface_2;
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, self.border_subtle);
        visuals.widgets.inactive.bg_fill = self.surface_2;
        visuals.widgets.inactive.fg_stroke.color = self.text_primary;
        visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, self.border_subtle);
        visuals.widgets.hovered.bg_fill = self.surface_3;
        visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, self.accent_hover);
        visuals.widgets.hovered.fg_stroke.color = self.text_primary;
        visuals.widgets.active.bg_fill = self.accent_muted;
        visuals.widgets.active.bg_stroke = Stroke::new(1.0, self.accent_primary);
        visuals.widgets.active.fg_stroke.color = self.text_primary;
        visuals.widgets.open.bg_fill = self.surface_3;
        visuals.widgets.open.bg_stroke = Stroke::new(1.0, self.border_subtle);
        visuals.selection.bg_fill = self.accent_muted;
        visuals.hyperlink_color = self.accent_primary;
        visuals.window_fill = self.surface_1;
        visuals.window_stroke = Stroke::new(1.0, self.border_subtle);
        visuals.window_corner_radius = CornerRadius::same(self.radius_10);
        visuals.window_shadow = egui::epaint::Shadow {
            offset: [0, 8],
            blur: 24,
            spread: 0,
            color: Color32::from_rgba_premultiplied(0, 0, 0, 64),
        };
        ctx.set_visuals(visuals);
    }

    pub fn panel_frame(&self, fill: Color32, inner_padding: i8) -> Frame {
        Frame::new()
            .fill(fill)
            .inner_margin(Margin::same(inner_padding))
            .corner_radius(CornerRadius::same(self.radius_10))
            .stroke(Stroke::new(1.0, self.border_subtle))
            .shadow(egui::epaint::Shadow {
                offset: [0, 4],
                blur: 20,
                spread: 0,
                color: Color32::from_rgba_premultiplied(0, 0, 0, 56),
            })
    }

    pub fn subtle_button_stroke(&self) -> Stroke {
        Stroke::new(1.0, self.border_subtle)
    }
}
