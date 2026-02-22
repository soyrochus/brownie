## 1) Make it feel modern/Material-like (egui)

### 1.1 Stop drawing heavy borders everywhere

Right now everything is outlined. Material uses elevation and surface separation, not constant strokes.

Instructions:

* Remove most `Frame::stroke(...)` usage.
* Use `Frame::group(ui.style())` (or `Frame::none()`) + fill color and rounding.
* Keep a thin stroke only for focused inputs and selected items.

In egui terms:

* Prefer `Frame::none().fill(surface_color).rounding(…)` and add spacing.
* Use shadows (subtle) if you’re on a recent egui that supports it; otherwise fake elevation with slightly different surface fills.

### 1.2 Define a proper theme with 4 surfaces + accent + text hierarchy

Add a `Theme` struct and forbid raw color literals outside theme.

Example theme values (dark, Material-ish):

```rust
pub struct Theme {
    pub bg: egui::Color32,        // app background
    pub surface1: egui::Color32,  // panels
    pub surface2: egui::Color32,  // cards / elevated
    pub surface3: egui::Color32,  // hover / active
    pub text: egui::Color32,
    pub text_muted: egui::Color32,
    pub accent: egui::Color32,
    pub accent_muted: egui::Color32,
    pub danger: egui::Color32,
    pub success: egui::Color32,
}

impl Default for Theme {
    fn default() -> Self {
        Self {
            bg: egui::Color32::from_rgb(15, 17, 21),
            surface1: egui::Color32::from_rgb(22, 26, 32),
            surface2: egui::Color32::from_rgb(28, 34, 43),
            surface3: egui::Color32::from_rgb(34, 42, 53),
            text: egui::Color32::from_rgb(230, 237, 243),
            text_muted: egui::Color32::from_rgb(139, 148, 158),
            accent: egui::Color32::from_rgb(59, 130, 246),
            accent_muted: egui::Color32::from_rgb(47, 110, 216),
            danger: egui::Color32::from_rgb(239, 68, 68),
            success: egui::Color32::from_rgb(34, 197, 94),
        }
    }
}
```

Then apply via `ctx.set_visuals(...)` and per-frame `ui.visuals_mut()` edits.

### 1.3 Apply consistent rounding + spacing scale

Material-like spacing is the bigger win than colors.

Instructions:

* Introduce constants: `R8 = 8.0`, `R12 = 12.0`, `P8 = 8.0`, `P12 = 12.0`, `P16 = 16.0`, `P24 = 24.0`.
* Every panel/card uses `rounding = 10–12`.
* Inner padding of cards: `12–16`.
* Vertical spacing between rows: `8–12`.

In egui:

* `ui.spacing_mut().item_spacing = vec2(10.0, 10.0);`
* `ui.spacing_mut().button_padding = vec2(12.0, 8.0);`
* Use `ui.add_space(…)` deliberately between sections, not random separators.

### 1.4 Use “cards” inside panels instead of flat content

Your “Canvas” area should be a surface with separate cards (like in the mock).

Implementation pattern:

* The right column is a `Frame` with `surface1`.
* Each widget group (metadata, decision, logs) is a `Frame` with `surface2`, rounded, padded.

```rust
let card = egui::Frame::none()
    .fill(theme.surface2)
    .rounding(egui::Rounding::same(12.0))
    .inner_margin(egui::Margin::same(12.0));

card.show(ui, |ui| {
    ui.label(egui::RichText::new("Review Metadata").strong());
    // …
});
```

### 1.5 Typography: stop using default sizes everywhere

Material feel comes from hierarchy.

Instructions:

* Titles: 16–18, semibold
* Section headers: 13–14, medium
* Body: 14
* Muted: 12–13

In egui:

* Use `RichText::size(…)` and `RichText::strong()`.
* Consider setting `ctx.set_style(Style { text_styles: … })` for global defaults.

### 1.6 Sidebar sessions: make them look like modern list items

Instructions:

* Each session item: rounded, subtle hover fill, selected state accent indicator (left bar or fill).
* Reduce borders; use background fill changes.

In egui you can render a session row as a custom `SelectableLabel` style:

* If selected: fill `surface3` and draw a small accent strip.

### 1.7 Buttons: introduce “primary” and “secondary” styles

Right now buttons look default.

Instructions:

* Primary button: filled accent, white text, rounded.
* Secondary: surface2 fill + subtle stroke.
* Destructive: danger tint.

Implementation approach in egui:

* Wrap in helper functions that return `Button` with `.fill()` and `.stroke()` based on style.

---

## 2) Multi-line chat input by default

You should use `egui::TextEdit::multiline` and handle Enter behavior.

Requirements:

* The input is 3–6 lines by default.
* `Enter` sends only if you choose a modifier (e.g., Ctrl+Enter). Plain Enter inserts newline.
* Provide a small “Send” button.

### 2.1 Minimal implementation

```rust
let send_on_ctrl_enter = true;

let mut send = false;

let te = egui::TextEdit::multiline(&mut self.chat_draft)
    .hint_text("Type a message…")
    .desired_rows(4)                 // default multi-line height
    .lock_focus(true)
    .desired_width(f32::INFINITY);

let resp = ui.add(te);

// Ctrl+Enter sends
if send_on_ctrl_enter && resp.has_focus() {
    ui.input(|i| {
        if i.key_pressed(egui::Key::Enter) && (i.modifiers.ctrl || i.modifiers.command) {
            send = true;
        }
    });
}

// Optional: keep newline on Enter by default (no extra handling)

ui.horizontal(|ui| {
    ui.add_space(4.0);
    if ui.add(primary_button("Send", &theme)).clicked() {
        send = true;
    }
});

if send {
    let msg = self.chat_draft.trim().to_string();
    if !msg.is_empty() {
        self.chat_draft.clear();
        self.send_message(msg);
    }
}
```

Notes:

* `desired_rows(4)` gives the “ChatGPT-ish” feel.
* `Ctrl+Enter` send is a good default for multiline; you avoid accidental sends.

### 2.2 Make it look like the draft

Wrap the input in a rounded “composer” frame:

```rust
egui::Frame::none()
    .fill(theme.surface2)
    .rounding(egui::Rounding::same(12.0))
    .inner_margin(egui::Margin::symmetric(12.0, 10.0))
    .show(ui, |ui| {
        ui.add(te);
        // footer row for Send and small toggles
    });
```

---

## Practical “do this first” order

1. Add `Theme` + remove scattered color literals.
2. Replace border-heavy frames with surface-based cards.
3. Fix spacing scale globally.
4. Convert chat input to multiline composer with Ctrl+Enter send.
5. Update sidebar session list styling and selected states.
