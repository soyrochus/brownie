## Goal

Refactor the current UI (brownie-ui.png) to visually resemble UI-draft.png **without modifying layout structure or component hierarchy**.

Do not change:

* Layout grid
* Component tree
* Interaction logic
* Panel positioning
* Component IDs

Only change:

* Color palette
* Typography scale
* Spacing and padding
* Borders and elevation
* Surface layering
* Icon and button styling
* Subtle gradients

---

# 1. Color System

### Replace flat dark background with layered dark surfaces.

Define a 4-level surface system:

```
Surface 0 (App background): #0F1115
Surface 1 (Panels):         #161A20
Surface 2 (Elevated blocks):#1C222B
Surface 3 (Active focus):   #222A35
```

### Accent color

Use a cool desaturated blue accent:

```
Primary Accent: #3B82F6
Hover Accent:   #4C8EF7
Muted Accent:   #2F6ED8
```

### Semantic colors

```
Success: #22C55E
Warning: #F59E0B
Danger:  #EF4444
Muted text: #8B949E
Primary text: #E6EDF3
```

Replace harsh white text with slightly softened off-white.

---

# 2. Spacing System

Adopt a consistent 8px spacing scale.

All paddings and margins must use:

```
4px
8px
12px
16px
24px
32px
```

Changes required:

* Increase left panel internal padding to 16px
* Increase chat message vertical spacing to 16px
* Increase canvas padding to 24px
* Add 12px spacing between action items
* Increase bottom input padding to 12px top/bottom

Remove cramped spacing.

---

# 3. Panel Styling

Current panels look flat and boxed.

Change to:

* Rounded corners: 10–12px radius
* Soft border: 1px solid rgba(255,255,255,0.05)
* Subtle elevation shadow:

```
box-shadow:
0px 1px 2px rgba(0,0,0,0.4),
0px 8px 24px rgba(0,0,0,0.25)
```

The draft UI has a layered glass-like depth — replicate subtle separation, not heavy borders.

---

# 4. Typography

Do not change font family unless necessary. Improve hierarchy:

### Title sizes

* Workspace title: 14px semibold
* Section headers: 13px medium
* Chat message body: 14px regular
* Code: 13px monospace
* Small metadata: 12px muted

Line height:

* 1.4–1.6 for text
* 1.3 for code

Remove bold overload. Use weight sparingly.

---

# 5. Chat Bubbles

Currently they look too flat.

Improve:

User message:

* Slight darker background (Surface 2)
* Rounded 12px
* Padding 12px
* Subtle right alignment offset

Assistant message:

* Slight lighter surface
* Left aligned
* Soft internal shadow

Add 8px vertical gap between bubbles.

---

# 6. Buttons

Replace default rectangular buttons with:

* Rounded 8px
* Height 34–36px
* Padding horizontal 14px
* Remove hard borders
* Use filled accent for primary
* Use muted outline for secondary

Primary button style:

Background: Primary Accent
Text: White
Hover: lighten accent slightly
Pressed: darken slightly

Secondary button:

Background: Surface 2
Border: 1px solid rgba(255,255,255,0.08)

---

# 7. Top Bar Refinement

Top bar should feel lighter and flatter.

* Slight gradient background from #161A20 to #14181E
* Remove heavy contrast
* Center connection status with small green dot
* Reduce vertical height slightly

Make it minimal.

---

# 8. Sidebar

Improve hierarchy:

* Workspace header slightly larger and brighter
* Session list items:

  * Rounded
  * Subtle hover highlight (Surface 3)
  * Active session highlighted with accent border-left (3px)

Spacing between list items: 6px.

---

# 9. Diff Component Styling

Current diff looks raw.

Improve:

* Use subtle background for added/removed lines:

  * Added: rgba(34,197,94,0.15)
  * Removed: rgba(239,68,68,0.15)
* Monospace font 13px
* Line padding 4px vertical
* Slight border-left indicator:

  * Green for additions
  * Red for deletions

Do not use strong red blocks.

---

# 10. Input Bar

Improve bottom input:

* Full-width rounded container
* Surface 2 background
* 1px subtle border
* Internal padding 12px
* Soft focus glow using accent color at 20% opacity

---

# 11. Overall Tone

Remove:

* Harsh contrast
* Pure black
* Hard white
* Strong borders

Introduce:

* Layered surfaces
* Soft shadows
* Muted text hierarchy
* Controlled accent usage
* Breathing room

The aesthetic goal is:

> Calm, restrained, high-clarity developer tool — not “dashboard”, not “neon”, not “consumer app”.

---

# 12. Implementation Constraints for Coding Agent

* Refactor styles into centralized theme module.
* No inline color literals after refactor.
* Introduce `Theme` struct with:

  * surface colors
  * semantic colors
  * spacing constants
  * radius constants
  * shadow presets
* Apply theme consistently across all components.

---

# Final Instruction to Agent

Do a pure aesthetic refactor pass.
No layout change.
No component tree modification.
No logic change.
No event change.
No ID change.

Only visual layer improvement.
