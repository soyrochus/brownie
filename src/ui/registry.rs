use crate::theme::Theme;
use crate::ui::event::{UiEvent, UiFieldValue};
use crate::ui::schema::{
    field_key, ButtonStyle, ComponentKind, DiffLineKind, FormFieldKind, SchemaRegistry,
    ValidatedComponent, ValidatedFormField,
};
use eframe::egui::{self, RichText};
use std::collections::{BTreeMap, BTreeSet};

pub struct ComponentRegistry {
    allowed_components: BTreeSet<&'static str>,
    allowed_field_kinds: BTreeSet<&'static str>,
}

impl ComponentRegistry {
    pub fn new() -> Self {
        Self {
            allowed_components: BTreeSet::from(["markdown", "form", "code", "diff", "button"]),
            allowed_field_kinds: BTreeSet::from(["text", "number", "select", "checkbox"]),
        }
    }

    pub fn render_component(
        &self,
        component: &ValidatedComponent,
        ui: &mut egui::Ui,
        theme: &Theme,
        form_state: &mut BTreeMap<String, UiFieldValue>,
        emit: &mut dyn FnMut(UiEvent),
    ) {
        match component {
            ValidatedComponent::Markdown(markdown) => {
                let frame = theme.card_frame();
                frame.show(ui, |ui| {
                    ui.label(
                        RichText::new(format!("id: {}", markdown.id))
                            .color(theme.text_muted)
                            .size(12.0),
                    );
                    ui.add_space(theme.spacing_4);
                    ui.label(
                        RichText::new(&markdown.text)
                            .color(theme.text_primary)
                            .size(14.0),
                    );
                });
                self.render_children(component, ui, theme, form_state, emit);
            }
            ValidatedComponent::Form(form) => {
                let frame = theme.card_frame();
                frame.show(ui, |ui| {
                    if let Some(title) = &form.title {
                        ui.label(RichText::new(title).color(theme.text_primary).size(13.0));
                        ui.add_space(theme.spacing_8);
                    }

                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = theme.spacing_12;
                        for field in &form.fields {
                            self.render_form_field(
                                form.id.as_str(),
                                field,
                                ui,
                                theme,
                                form_state,
                                emit,
                            );
                        }
                    });
                });
                self.render_children(component, ui, theme, form_state, emit);
            }
            ValidatedComponent::Code(code) => {
                let frame = theme.card_frame();
                frame.show(ui, |ui| {
                    ui.label(
                        RichText::new(format!("id: {}", code.id))
                            .color(theme.text_muted)
                            .size(12.0),
                    );
                    ui.add_space(theme.spacing_4);
                    let language = code.language.as_deref().unwrap_or("code");
                    ui.label(RichText::new(language).color(theme.text_muted).size(12.0));
                    ui.add_space(theme.spacing_8);
                    ui.label(
                        RichText::new(code.code.as_str())
                            .color(theme.text_primary)
                            .size(13.0)
                            .monospace(),
                    );
                });
                self.render_children(component, ui, theme, form_state, emit);
            }
            ValidatedComponent::Diff(diff) => {
                let frame = theme.card_frame();
                frame.show(ui, |ui| {
                    ui.label(
                        RichText::new(format!("id: {}", diff.id))
                            .color(theme.text_muted)
                            .size(12.0),
                    );
                    ui.add_space(theme.spacing_4);
                    for line in &diff.lines {
                        let (fill, accent) = match line.kind {
                            DiffLineKind::Added => (theme.diff_added_tint, theme.success),
                            DiffLineKind::Removed => (theme.diff_removed_tint, theme.danger),
                            DiffLineKind::Context => (theme.surface_3, theme.border_subtle),
                        };
                        egui::Frame::new()
                            .fill(fill)
                            .stroke(egui::Stroke::NONE)
                            .corner_radius(egui::CornerRadius::same(theme.radius_8))
                            .inner_margin(egui::Margin::symmetric(
                                theme.spacing_8 as i8,
                                theme.spacing_4 as i8,
                            ))
                            .show(ui, |ui| {
                                ui.horizontal(|ui| {
                                    ui.colored_label(accent, "▌");
                                    ui.label(
                                        RichText::new(&line.text)
                                            .color(theme.text_primary)
                                            .size(13.0)
                                            .monospace(),
                                    );
                                });
                            });
                    }
                });
                self.render_children(component, ui, theme, form_state, emit);
            }
            ValidatedComponent::Button(button) => {
                let (fill, stroke, text_color) = match button.variant {
                    ButtonStyle::Primary => (
                        theme.accent_primary,
                        theme.primary_button_stroke(),
                        theme.text_on_accent,
                    ),
                    ButtonStyle::Secondary => (
                        theme.surface_2,
                        theme.subtle_button_stroke(),
                        theme.text_primary,
                    ),
                };
                let button_widget =
                    egui::Button::new(RichText::new(&button.label).color(text_color).size(13.0))
                        .fill(fill)
                        .stroke(stroke)
                        .corner_radius(egui::CornerRadius::same(theme.radius_8))
                        .min_size(egui::vec2(0.0, theme.button_height));

                if ui.add(button_widget).clicked() {
                    emit(UiEvent::ButtonClicked {
                        component_id: button.id.clone(),
                        output_event_id: button.output_event_id.clone(),
                    });
                }

                self.render_children(component, ui, theme, form_state, emit);
            }
        }
    }

    fn render_children(
        &self,
        component: &ValidatedComponent,
        ui: &mut egui::Ui,
        theme: &Theme,
        form_state: &mut BTreeMap<String, UiFieldValue>,
        emit: &mut dyn FnMut(UiEvent),
    ) {
        for child in component.children() {
            ui.add_space(theme.spacing_8);
            self.render_component(child, ui, theme, form_state, emit);
        }
    }

    fn render_form_field(
        &self,
        form_id: &str,
        field: &ValidatedFormField,
        ui: &mut egui::Ui,
        theme: &Theme,
        form_state: &mut BTreeMap<String, UiFieldValue>,
        emit: &mut dyn FnMut(UiEvent),
    ) {
        let field_id = field.id().to_string();
        let state_key = field_key(form_id, &field_id);
        let current = form_state
            .entry(state_key.clone())
            .or_insert_with(|| field.default_value())
            .clone();

        match field {
            ValidatedFormField::Text(text_field) => {
                let mut value = match current {
                    UiFieldValue::Text { value } => value,
                    _ => text_field.default.clone(),
                };
                ui.label(
                    RichText::new(&text_field.label)
                        .color(theme.text_muted)
                        .size(12.0),
                );
                let response = ui.add(
                    egui::TextEdit::singleline(&mut value)
                        .desired_width(f32::INFINITY)
                        .hint_text("text"),
                );
                if response.lost_focus() && response.changed() {
                    let value = UiFieldValue::Text { value };
                    form_state.insert(state_key, value.clone());
                    emit(UiEvent::FormFieldCommitted {
                        component_id: form_id.to_string(),
                        form_id: form_id.to_string(),
                        field_id,
                        value,
                    });
                } else {
                    form_state.insert(state_key, UiFieldValue::Text { value });
                }
            }
            ValidatedFormField::Number(number_field) => {
                let mut value = match current {
                    UiFieldValue::Number { value } => value,
                    _ => number_field.default,
                };
                ui.label(
                    RichText::new(&number_field.label)
                        .color(theme.text_muted)
                        .size(12.0),
                );
                let response = ui.add(egui::DragValue::new(&mut value).speed(0.1));
                if response.changed() {
                    let value = UiFieldValue::Number { value };
                    form_state.insert(state_key, value.clone());
                    emit(UiEvent::FormFieldCommitted {
                        component_id: form_id.to_string(),
                        form_id: form_id.to_string(),
                        field_id,
                        value,
                    });
                }
            }
            ValidatedFormField::Select(select_field) => {
                let mut value = match current {
                    UiFieldValue::Select { value } => value,
                    _ => select_field.default.clone(),
                };
                ui.label(
                    RichText::new(&select_field.label)
                        .color(theme.text_muted)
                        .size(12.0),
                );
                let mut changed = false;
                egui::ComboBox::from_id_salt(state_key.clone())
                    .selected_text(value.clone())
                    .show_ui(ui, |ui| {
                        for option in &select_field.options {
                            if ui
                                .selectable_value(&mut value, option.clone(), option)
                                .changed()
                            {
                                changed = true;
                            }
                        }
                    });
                if changed {
                    let value = UiFieldValue::Select { value };
                    form_state.insert(state_key, value.clone());
                    emit(UiEvent::FormFieldCommitted {
                        component_id: form_id.to_string(),
                        form_id: form_id.to_string(),
                        field_id,
                        value,
                    });
                }
            }
            ValidatedFormField::Checkbox(checkbox_field) => {
                let mut checked = match current {
                    UiFieldValue::Checkbox { value } => value,
                    _ => checkbox_field.default,
                };
                if ui
                    .checkbox(
                        &mut checked,
                        RichText::new(&checkbox_field.label)
                            .color(theme.text_primary)
                            .size(13.0),
                    )
                    .changed()
                {
                    let value = UiFieldValue::Checkbox { value: checked };
                    form_state.insert(state_key, value.clone());
                    emit(UiEvent::FormFieldCommitted {
                        component_id: form_id.to_string(),
                        form_id: form_id.to_string(),
                        field_id,
                        value,
                    });
                }
            }
        }
    }
}

impl SchemaRegistry for ComponentRegistry {
    fn supports_component(&self, kind: &ComponentKind) -> bool {
        self.allowed_components.contains(kind.as_str())
    }

    fn supports_field_kind(&self, kind: &FormFieldKind) -> bool {
        self.allowed_field_kinds.contains(kind.as_str())
    }
}
