use super::appkit_sys::{NsPoint, NsRect, NsSize};
use super::scene_state::{NativeSceneNode, NativeSceneState};
use crate::engine::{NodeId, StyleRule};
use std::collections::HashMap;

/// Computed screen rectangle (in logical DIP / points) produced by the shared Rust layout solver.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LayoutRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl LayoutRect {
    /// Converts a computed layout rectangle into Cocoa's `NsRect` (`CGRect`).
    #[must_use]
    pub const fn to_ns_rect(self) -> NsRect {
        NsRect {
            origin: NsPoint {
                x: self.x as f64,
                y: self.y as f64,
            },
            size: NsSize {
                width: if self.width > 0 {
                    self.width as f64
                } else {
                    0.0
                },
                height: if self.height > 0 {
                    self.height as f64
                } else {
                    0.0
                },
            },
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Axis {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Align {
    Fill,
    Start,
    Center,
    End,
}

impl Align {
    fn parse(raw: &str) -> Self {
        match raw.trim().to_ascii_lowercase().as_str() {
            "start" => Self::Start,
            "center" => Self::Center,
            "end" => Self::End,
            _ => Self::Fill,
        }
    }
}

#[derive(Debug, Clone)]
struct EffectiveLayoutProps {
    padding: i32,
    spacing: i32,
    margin: i32,
    width: Option<i32>,
    height: Option<i32>,
    min_width: i32,
    min_height: i32,
    max_width: Option<i32>,
    orientation: Option<Axis>,
    hexpand: bool,
    vexpand: bool,
    halign: Align,
    valign: Align,
    homogeneous: bool,
    columns: Option<usize>,
    split_position: Option<i32>,
    grid_col: Option<usize>,
    grid_row: Option<usize>,
}

fn parse_px(raw: &str) -> Option<i32> {
    raw.trim()
        .trim_end_matches("px")
        .trim()
        .parse::<i32>()
        .ok()
        .map(|v| v.max(0))
}

fn parse_axis(raw: &str) -> Option<Axis> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "horizontal" | "row" => Some(Axis::Horizontal),
        "vertical" | "column" => Some(Axis::Vertical),
        _ => None,
    }
}

fn is_visible_node(node: &NativeSceneNode) -> bool {
    if node.native_class.contains("AnchorComment") {
        return false;
    }
    if node.native_class == "Microsoft.UI.Xaml.Controls.Flyout" || node.native_class == "NSPopover"
    {
        return node
            .attributes
            .get("open")
            .is_some_and(|v| NativeSceneState::parse_truthy_attr(v));
    }
    node.attributes
        .get("visible")
        .is_none_or(|v| NativeSceneState::parse_truthy_attr(v))
}

fn default_intrinsic_size(node: &NativeSceneNode) -> (i32, i32) {
    match node.native_class {
        "Microsoft.UI.Xaml.Controls.TitleBar" | "NSToolbar" => (320, 44),
        "Microsoft.UI.Xaml.Controls.TextBox"
        | "Microsoft.UI.Xaml.Controls.PasswordBox"
        | "Microsoft.UI.Xaml.Controls.AutoSuggestBox"
        | "Microsoft.UI.Xaml.Controls.ComboBox"
        | "Microsoft.UI.Xaml.Controls.NumberBox"
        | "NSTextFieldEditable"
        | "NSSecureTextField"
        | "NSSearchField"
        | "NSPopUpButton"
        | "NSStepper"
        | "Microsoft.UI.Xaml.Controls.Button"
        | "Microsoft.UI.Xaml.Controls.CheckBox"
        | "Microsoft.UI.Xaml.Controls.ToggleSwitch"
        | "NSButton"
        | "NSButtonCheckbox"
        | "NSSwitch" => (180, 36),
        "Microsoft.UI.Xaml.Controls.RichEditBox" | "NSTextView" => (320, 120),
        "Microsoft.UI.Xaml.Controls.AppBarSeparator" | "NSBoxSeparator" => (240, 2),
        "Microsoft.UI.Xaml.Controls.Slider"
        | "Microsoft.UI.Xaml.Controls.ProgressBar"
        | "NSSlider"
        | "NSLevelIndicator"
        | "NSProgressIndicator" => (220, 24),
        "Microsoft.UI.Xaml.Controls.Image" | "NSImageView" => (160, 160),
        "Microsoft.UI.Xaml.Controls.StackPanel"
        | "Microsoft.UI.Xaml.Controls.Grid"
        | "Microsoft.UI.Xaml.Controls.ScrollViewer"
        | "Microsoft.UI.Xaml.Controls.SplitView"
        | "Microsoft.UI.Xaml.Controls.ItemsRepeater"
        | "Microsoft.UI.Xaml.Controls.ListView"
        | "NSStackView"
        | "NSGridView"
        | "NSScrollView"
        | "NSSplitView"
        | "NSCollectionView"
        | "NSTableView"
        | "NSBox" => (360, 140),
        _ => (240, 32),
    }
}

fn apply_style_rule(props: &mut EffectiveLayoutProps, rule: &StyleRule) {
    if rule.padding > 0 {
        props.padding = i32::from(rule.padding);
    }
    if rule.spacing > 0 {
        props.spacing = i32::from(rule.spacing);
    }
    if let Some(mw) = rule.min_width {
        props.min_width = i32::from(mw);
    }
    if let Some(mh) = rule.min_height {
        props.min_height = i32::from(mh);
    }
    if let Some(axis) = rule.orientation.as_deref().and_then(parse_axis) {
        props.orientation = Some(axis);
    }
    if let Some(hx) = rule.hexpand {
        props.hexpand = hx;
    }
    if let Some(vx) = rule.vexpand {
        props.vexpand = vx;
    }
    if let Some(ha) = rule.halign.as_deref() {
        props.halign = Align::parse(ha);
    }
    if let Some(va) = rule.valign.as_deref() {
        props.valign = Align::parse(va);
    }
    if let Some(hom) = rule.homogeneous {
        props.homogeneous = hom;
    }
    if let Some(mc) = rule.max_children {
        props.columns = Some(usize::from(mc).max(1));
    }
}

fn apply_attr_override(props: &mut EffectiveLayoutProps, key: &str, val: &str) {
    match key {
        "padding" => props.padding = parse_px(val).unwrap_or(props.padding),
        "spacing" | "gap" => props.spacing = parse_px(val).unwrap_or(props.spacing),
        "margin" => props.margin = parse_px(val).unwrap_or(props.margin),
        "width" => props.width = parse_px(val),
        "height" => props.height = parse_px(val),
        "min-width" => props.min_width = parse_px(val).unwrap_or(props.min_width),
        "min-height" => props.min_height = parse_px(val).unwrap_or(props.min_height),
        "max-width" | "max-size" | "maximum-size" => props.max_width = parse_px(val),
        "orientation" | "flex-direction" => {
            if let Some(axis) = parse_axis(val) {
                props.orientation = Some(axis);
            }
        }
        "hexpand" => props.hexpand = NativeSceneState::parse_truthy_attr(val),
        "vexpand" => props.vexpand = NativeSceneState::parse_truthy_attr(val),
        "halign" => props.halign = Align::parse(val),
        "valign" => props.valign = Align::parse(val),
        "homogeneous" => props.homogeneous = NativeSceneState::parse_truthy_attr(val),
        "columns" | "max-children" => {
            if let Ok(c) = val.trim().parse::<usize>() {
                props.columns = Some(c.max(1));
            }
        }
        "position" => props.split_position = parse_px(val),
        "col" | "column" => props.grid_col = val.trim().parse::<usize>().ok(),
        "row" => props.grid_row = val.trim().parse::<usize>().ok(),
        _ => {}
    }
}

fn resolve_props(state: &NativeSceneState, node: &NativeSceneNode) -> EffectiveLayoutProps {
    let is_toolbar = matches!(
        node.native_class,
        "Microsoft.UI.Xaml.Controls.TitleBar" | "NSToolbar"
    );
    let is_toplevel = matches!(
        node.native_class,
        "Microsoft.UI.Xaml.Window"
            | "Microsoft.UI.Xaml.Controls.ContentDialog"
            | "NSWindow"
            | "NSPanel"
    );
    let mut props = EffectiveLayoutProps {
        padding: if is_toplevel { 12 } else { 0 },
        spacing: if is_toolbar { 8 } else { 6 },
        margin: 0,
        width: None,
        height: None,
        min_width: 0,
        min_height: 0,
        max_width: matches!(node.source_tag.as_str(), "ng-clamp" | "adw-clamp").then_some(680),
        orientation: if is_toolbar {
            Some(Axis::Horizontal)
        } else {
            None
        },
        hexpand: false,
        vexpand: false,
        halign: Align::Fill,
        valign: Align::Fill,
        homogeneous: false,
        columns: None,
        split_position: None,
        grid_col: None,
        grid_row: None,
    };

    for &style_id in &node.style_classes {
        if let Some(rule) = state.get_style(style_id) {
            apply_style_rule(&mut props, rule);
        }
    }
    for (key, val) in &node.attributes {
        apply_attr_override(&mut props, key, val);
    }
    props
}

fn align_span(available: i32, desired: i32, align: Align) -> (i32, i32) {
    let clamped = desired.clamp(1, available.max(1));
    match align {
        Align::Fill => (0, available.max(1)),
        Align::Start => (0, clamped),
        Align::Center => ((available - clamped).max(0) / 2, clamped),
        Align::End => ((available - clamped).max(0), clamped),
    }
}

fn container_inner_rect(outer: LayoutRect, props: &EffectiveLayoutProps) -> LayoutRect {
    let inset = props.padding.max(0);
    let mut inner_w = (outer.width - inset * 2).max(1);
    let mut inner_x = outer.x + inset;
    if let Some(max_w) = props.max_width
        && inner_w > max_w.max(1)
    {
        let capped = max_w.max(1);
        inner_x += (inner_w - capped) / 2;
        inner_w = capped;
    }
    LayoutRect {
        x: inner_x,
        y: outer.y + inset,
        width: inner_w,
        height: (outer.height - inset * 2).max(1),
    }
}

fn measure_base_child(
    state: &NativeSceneState,
    child: &NativeSceneNode,
) -> (EffectiveLayoutProps, i32, i32) {
    let props = resolve_props(state, child);
    let (def_w, def_h) = default_intrinsic_size(child);
    let base_w = props.width.unwrap_or(def_w).max(props.min_width).max(1);
    let base_h = props.height.unwrap_or(def_h).max(props.min_height).max(1);
    (props, base_w, base_h)
}

struct MeasuredFlexChild {
    id: NodeId,
    props: EffectiveLayoutProps,
    base_w: i32,
    base_h: i32,
    expands: bool,
}

fn measure_flex_children(
    state: &NativeSceneState,
    axis: Axis,
    homogeneous: bool,
    children: &[NodeId],
) -> (Vec<MeasuredFlexChild>, i32, i32) {
    let mut measured = Vec::with_capacity(children.len());
    let mut fixed_sum = 0i32;
    let mut flex_count = 0i32;

    for &cid in children {
        let Some(cnode) = state.get_node(cid) else {
            continue;
        };
        let (props, base_w, base_h) = measure_base_child(state, cnode);
        let (base_main, expands) = match axis {
            Axis::Vertical => (base_h, props.vexpand || homogeneous),
            Axis::Horizontal => (base_w, props.hexpand || homogeneous),
        };
        if homogeneous {
            flex_count += 1;
        } else if expands {
            flex_count += 1;
            fixed_sum = fixed_sum.saturating_add(base_main);
        } else {
            fixed_sum = fixed_sum.saturating_add(base_main);
        }
        measured.push(MeasuredFlexChild {
            id: cid,
            props,
            base_w,
            base_h,
            expands,
        });
    }
    (measured, fixed_sum, flex_count)
}

fn layout_flex_children(
    state: &NativeSceneState,
    inner: LayoutRect,
    parent_props: &EffectiveLayoutProps,
    children: &[NodeId],
    out: &mut HashMap<NodeId, LayoutRect>,
) {
    let count = i32::try_from(children.len()).unwrap_or(0);
    if count == 0 {
        return;
    }
    let axis = parent_props.orientation.unwrap_or(Axis::Vertical);
    let gap = parent_props.spacing.max(0);
    let total_gap = gap.saturating_mul(count - 1);
    let main_avail = match axis {
        Axis::Vertical => (inner.height - total_gap).max(count),
        Axis::Horizontal => (inner.width - total_gap).max(count),
    };

    let (measured, fixed_sum, flex_count) =
        measure_flex_children(state, axis, parent_props.homogeneous, children);

    let extra = if parent_props.homogeneous {
        0
    } else {
        (main_avail - fixed_sum).max(0)
    };
    let each_extra = if !parent_props.homogeneous && flex_count > 0 {
        extra / flex_count
    } else {
        0
    };
    let mut remainder = if !parent_props.homogeneous && flex_count > 0 {
        extra % flex_count
    } else {
        0
    };
    let homo_each = if parent_props.homogeneous && flex_count > 0 {
        main_avail / flex_count
    } else {
        0
    };
    let mut homo_rem = if parent_props.homogeneous && flex_count > 0 {
        main_avail % flex_count
    } else {
        0
    };

    let mut cursor = match axis {
        Axis::Vertical => inner.y,
        Axis::Horizontal => inner.x,
    };

    for item in measured {
        let mut child_main = if parent_props.homogeneous {
            let bonus = i32::from(homo_rem > 0);
            if homo_rem > 0 {
                homo_rem -= 1;
            }
            homo_each + bonus
        } else if item.expands {
            let base = match axis {
                Axis::Vertical => item.base_h,
                Axis::Horizontal => item.base_w,
            };
            let bonus = i32::from(remainder > 0);
            if remainder > 0 {
                remainder -= 1;
            }
            base + each_extra + bonus
        } else {
            match axis {
                Axis::Vertical => item.base_h,
                Axis::Horizontal => item.base_w,
            }
        };

        let margin = item.props.margin.max(0);
        child_main = (child_main - margin * 2).max(1);

        let rect = match axis {
            Axis::Vertical => {
                let avail_w = (inner.width - margin * 2).max(1);
                let (dx, w) = align_span(avail_w, item.base_w, item.props.halign);
                LayoutRect {
                    x: inner.x + margin + dx,
                    y: cursor + margin,
                    width: w,
                    height: child_main,
                }
            }
            Axis::Horizontal => {
                let avail_h = (inner.height - margin * 2).max(1);
                let (dy, h) = align_span(avail_h, item.base_h, item.props.valign);
                LayoutRect {
                    x: cursor + margin,
                    y: inner.y + margin + dy,
                    width: child_main,
                    height: h,
                }
            }
        };

        cursor += child_main + margin * 2 + gap;
        layout_subtree(state, item.id, rect, out);
    }
}

fn layout_grid_children(
    state: &NativeSceneState,
    inner: LayoutRect,
    spacing: i32,
    cols: usize,
    children: &[NodeId],
    out: &mut HashMap<NodeId, LayoutRect>,
) {
    if children.is_empty() {
        return;
    }
    let col_count = cols.max(1);
    let gap = spacing.max(0);

    let mut placements = Vec::with_capacity(children.len());
    let mut max_row = 0usize;
    for (idx, &cid) in children.iter().enumerate() {
        let Some(cnode) = state.get_node(cid) else {
            continue;
        };
        let props = resolve_props(state, cnode);
        let col = props.grid_col.unwrap_or(idx % col_count).min(col_count - 1);
        let row = props.grid_row.unwrap_or(idx / col_count);
        max_row = max_row.max(row);
        placements.push((cid, props, col, row));
    }

    let row_count = max_row + 1;
    let total_cols = i32::try_from(col_count).unwrap_or(1);
    let total_rows = i32::try_from(row_count).unwrap_or(1);
    let cell_w = ((inner.width - gap.saturating_mul(total_cols - 1)) / total_cols).max(1);
    let cell_h = ((inner.height - gap.saturating_mul(total_rows - 1)) / total_rows).max(1);

    for (cid, props, col, row) in placements {
        let margin = props.margin.max(0);
        let x_idx = i32::try_from(col).unwrap_or(0);
        let y_idx = i32::try_from(row).unwrap_or(0);
        let rect = LayoutRect {
            x: inner.x + x_idx * (cell_w + gap) + margin,
            y: inner.y + y_idx * (cell_h + gap) + margin,
            width: (cell_w - margin * 2).max(1),
            height: (cell_h - margin * 2).max(1),
        };
        layout_subtree(state, cid, rect, out);
    }
}

fn layout_paned_children(
    state: &NativeSceneState,
    inner: LayoutRect,
    parent_props: &EffectiveLayoutProps,
    children: &[NodeId],
    out: &mut HashMap<NodeId, LayoutRect>,
) {
    if children.is_empty() {
        return;
    }
    if children.len() == 1 {
        layout_subtree(state, children[0], inner, out);
        return;
    }
    let axis = parent_props.orientation.unwrap_or(Axis::Horizontal);
    let gap = parent_props.spacing.max(4);
    let total_main = match axis {
        Axis::Horizontal => inner.width,
        Axis::Vertical => inner.height,
    };
    let first_size = parent_props
        .split_position
        .unwrap_or(total_main / 3)
        .clamp(40, (total_main - gap - 40).max(40));
    let second_size = (total_main - first_size - gap).max(1);

    let (r1, r2) = match axis {
        Axis::Horizontal => (
            LayoutRect {
                x: inner.x,
                y: inner.y,
                width: first_size,
                height: inner.height,
            },
            LayoutRect {
                x: inner.x + first_size + gap,
                y: inner.y,
                width: second_size,
                height: inner.height,
            },
        ),
        Axis::Vertical => (
            LayoutRect {
                x: inner.x,
                y: inner.y,
                width: inner.width,
                height: first_size,
            },
            LayoutRect {
                x: inner.x,
                y: inner.y + first_size + gap,
                width: inner.width,
                height: second_size,
            },
        ),
    };
    layout_subtree(state, children[0], r1, out);
    layout_subtree(state, children[1], r2, out);
    for &extra in &children[2..] {
        layout_subtree(state, extra, r2, out);
    }
}

fn layout_subtree(
    state: &NativeSceneState,
    node_id: NodeId,
    rect: LayoutRect,
    out: &mut HashMap<NodeId, LayoutRect>,
) {
    let Some(node) = state.get_node(node_id) else {
        return;
    };
    if !is_visible_node(node) {
        return;
    }
    out.insert(node_id, rect);

    let visible_children: Vec<NodeId> = node
        .children
        .iter()
        .copied()
        .filter(|cid| state.get_node(*cid).is_some_and(is_visible_node))
        .collect();
    if visible_children.is_empty() {
        return;
    }

    let props = resolve_props(state, node);
    let inner = container_inner_rect(rect, &props);
    if matches!(node.source_tag.as_str(), "ng-clamp" | "adw-clamp") {
        layout_flex_children(state, inner, &props, &visible_children, out);
        return;
    }
    match node.native_class {
        "Microsoft.UI.Xaml.Controls.Grid" | "NSGridView" => {
            layout_grid_children(
                state,
                inner,
                props.spacing,
                props.columns.unwrap_or(2),
                &visible_children,
                out,
            );
        }
        "Microsoft.UI.Xaml.Controls.ItemsRepeater" | "NSCollectionView" => {
            layout_grid_children(
                state,
                inner,
                props.spacing,
                props.columns.unwrap_or(3),
                &visible_children,
                out,
            );
        }
        "Microsoft.UI.Xaml.Controls.SplitView" | "NSSplitView" => {
            layout_paned_children(state, inner, &props, &visible_children, out);
        }
        _ => {
            layout_flex_children(state, inner, &props, &visible_children, out);
        }
    }
}

/// Solves CSS Flexbox/Grid/Paned layout coordinates across an entire scene tree rooted at `root_id`.
#[must_use]
pub fn solve_scene_layout(
    state: &NativeSceneState,
    root_id: NodeId,
    viewport_width: i32,
    viewport_height: i32,
) -> HashMap<NodeId, LayoutRect> {
    let mut out = HashMap::new();
    let Some(root_node) = state.get_node(root_id) else {
        return out;
    };
    let props = resolve_props(state, root_node);
    let root_rect = LayoutRect {
        x: 0,
        y: 0,
        width: props.width.unwrap_or(viewport_width).max(160),
        height: props.height.unwrap_or(viewport_height).max(120),
    };
    layout_subtree(state, root_id, root_rect, &mut out);
    out
}

/// Converts a root-relative `LayoutRect` into coordinates relative to its immediate parent,
/// optionally flipping the vertical axis for unflipped bottom-up Cocoa `NSView` frames.
#[must_use]
pub fn to_parent_relative_rect(
    parent: Option<LayoutRect>,
    child: LayoutRect,
    flip_y_for_cocoa: bool,
) -> LayoutRect {
    let (px, py, ph) = parent.map_or((0, 0, child.height), |p| (p.x, p.y, p.height));
    let rel_x = child.x - px;
    let rel_y = if flip_y_for_cocoa {
        (ph - (child.y - py) - child.height).max(0)
    } else {
        child.y - py
    };
    LayoutRect {
        x: rel_x,
        y: rel_y,
        width: child.width,
        height: child.height,
    }
}

/// Computes parent-relative bounds for every laid-out child node given its root-relative rectangles.
#[must_use]
pub fn compute_parent_relative_bounds<S: std::hash::BuildHasher>(
    state: &NativeSceneState,
    root_rects: &HashMap<NodeId, LayoutRect, S>,
    flip_y_for_cocoa: bool,
) -> HashMap<NodeId, LayoutRect> {
    let mut parent_by_child = HashMap::new();
    for (&node_id, rect) in root_rects {
        if let Some(node) = state.get_node(node_id) {
            for &child in &node.children {
                parent_by_child.insert(child, *rect);
            }
        }
    }
    root_rects
        .iter()
        .map(|(&id, &child_rect)| {
            let rel = to_parent_relative_rect(
                parent_by_child.get(&id).copied(),
                child_rect,
                flip_y_for_cocoa,
            );
            (id, rel)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::{RenderCommand, StyleId};

    const fn nid(index: usize) -> NodeId {
        NodeId {
            index,
            generation: 1,
        }
    }

    fn set_attrs(state: &mut NativeSceneState, node: NodeId, pairs: &[(&str, &str)]) {
        for &(k, v) in pairs {
            let cmd = RenderCommand::SetAttribute {
                node,
                key: String::from(k),
                value: String::from(v),
            };
            state.apply_command(&cmd, "AnchorComment", |_| "NSWindow");
        }
    }

    fn create_widget(state: &mut NativeSceneState, id: NodeId, tag: &str) {
        state.apply_command(
            &RenderCommand::CreateWidget {
                id,
                tag: String::from(tag),
            },
            "AnchorComment",
            |t| match t {
                "ng-window" => "NSWindow",
                "ng-box" => "NSStackView",
                "ng-button" => "NSButton",
                "ng-grid" => "NSGridView",
                "ng-paned" => "NSSplitView",
                _ => "NSTextField",
            },
        );
    }

    fn append(state: &mut NativeSceneState, parent: NodeId, child: NodeId) {
        state.apply_command(
            &RenderCommand::AppendChild { parent, child },
            "AnchorComment",
            |_| "NSWindow",
        );
    }

    #[test]
    fn solves_flexbox_vertical_and_horizontal_expand_and_alignment() {
        let mut state = NativeSceneState::new();
        let (win, root_box, title, row) = (nid(0), nid(1), nid(2), nid(3));
        let (side, spacer_lbl, comment, hidden) = (nid(4), nid(5), nid(6), nid(7));

        create_widget(&mut state, win, "ng-window");
        set_attrs(&mut state, win, &[("padding", "10")]);
        create_widget(&mut state, root_box, "ng-box");
        set_attrs(
            &mut state,
            root_box,
            &[("vexpand", "true"), ("spacing", "10")],
        );
        create_widget(&mut state, title, "ng-label");
        set_attrs(
            &mut state,
            title,
            &[("height", "40"), ("width", "200"), ("halign", "center")],
        );
        create_widget(&mut state, row, "ng-box");
        set_attrs(
            &mut state,
            row,
            &[
                ("orientation", "horizontal"),
                ("vexpand", "true"),
                ("spacing", "8"),
            ],
        );
        create_widget(&mut state, side, "ng-button");
        set_attrs(&mut state, side, &[("width", "120")]);
        create_widget(&mut state, spacer_lbl, "ng-label");
        set_attrs(&mut state, spacer_lbl, &[("hexpand", "true")]);
        state.apply_command(
            &RenderCommand::CreateComment {
                id: comment,
                text: String::from("@if"),
            },
            "AnchorComment",
            |_| "AnchorComment",
        );
        create_widget(&mut state, hidden, "ng-label");
        set_attrs(&mut state, hidden, &[("visible", "false")]);

        for &(p, c) in &[
            (win, root_box),
            (root_box, comment),
            (root_box, title),
            (root_box, hidden),
            (root_box, row),
            (row, side),
            (row, spacer_lbl),
        ] {
            append(&mut state, p, c);
        }

        let rects = solve_scene_layout(&state, win, 820, 620);
        assert!(!rects.contains_key(&comment));
        assert!(!rects.contains_key(&hidden));
        assert_eq!(
            rects[&root_box],
            LayoutRect {
                x: 10,
                y: 10,
                width: 800,
                height: 600
            }
        );
        assert_eq!(
            rects[&title],
            LayoutRect {
                x: 310,
                y: 10,
                width: 200,
                height: 40
            }
        );
        assert_eq!(
            rects[&row],
            LayoutRect {
                x: 10,
                y: 60,
                width: 800,
                height: 550
            }
        );
        assert_eq!(rects[&side].width, 120);
        assert_eq!(rects[&spacer_lbl].x, 138);
        assert_eq!(rects[&spacer_lbl].width, 672);
    }

    #[test]
    fn solves_grid_paned_and_clamp_layouts() {
        let mut state = NativeSceneState::new();
        let grid = nid(0);
        let a = nid(1);
        let b = nid(2);
        let c = nid(3);
        let paned = nid(4);
        let left = nid(5);
        let right = nid(6);

        for cmd in [
            RenderCommand::CreateWidget {
                id: grid,
                tag: String::from("ng-grid"),
            },
            RenderCommand::SetAttribute {
                node: grid,
                key: String::from("columns"),
                value: String::from("2"),
            },
            RenderCommand::SetAttribute {
                node: grid,
                key: String::from("spacing"),
                value: String::from("10"),
            },
            RenderCommand::CreateWidget {
                id: a,
                tag: String::from("ng-label"),
            },
            RenderCommand::CreateWidget {
                id: b,
                tag: String::from("ng-label"),
            },
            RenderCommand::CreateWidget {
                id: c,
                tag: String::from("ng-label"),
            },
            RenderCommand::AppendChild {
                parent: grid,
                child: a,
            },
            RenderCommand::AppendChild {
                parent: grid,
                child: b,
            },
            RenderCommand::AppendChild {
                parent: grid,
                child: c,
            },
            RenderCommand::CreateWidget {
                id: paned,
                tag: String::from("ng-paned"),
            },
            RenderCommand::SetAttribute {
                node: paned,
                key: String::from("position"),
                value: String::from("220"),
            },
            RenderCommand::CreateWidget {
                id: left,
                tag: String::from("ng-label"),
            },
            RenderCommand::CreateWidget {
                id: right,
                tag: String::from("ng-label"),
            },
            RenderCommand::AppendChild {
                parent: paned,
                child: left,
            },
            RenderCommand::AppendChild {
                parent: paned,
                child: right,
            },
        ] {
            state.apply_command(&cmd, "AnchorComment", |t| match t {
                "ng-grid" => "NSGridView",
                "ng-paned" => "NSSplitView",
                _ => "NSTextField",
            });
        }

        let grid_rects = solve_scene_layout(&state, grid, 410, 210);
        assert_eq!(grid_rects[&a].width, 200);
        assert_eq!(grid_rects[&b].x, 210);
        assert_eq!(grid_rects[&c].y, 110);

        let paned_rects = solve_scene_layout(&state, paned, 800, 500);
        assert_eq!(paned_rects[&left].width, 220);
        assert_eq!(paned_rects[&right].x, 226);
        assert_eq!(paned_rects[&right].width, 574);
    }

    #[test]
    fn converts_root_rects_to_parent_relative_for_win32_and_cocoa() {
        let mut state = NativeSceneState::new();
        let win = nid(0);
        let box_node = nid(1);
        let child = nid(2);

        create_widget(&mut state, win, "ng-window");
        set_attrs(&mut state, win, &[("padding", "10")]);
        create_widget(&mut state, box_node, "ng-box");
        set_attrs(
            &mut state,
            box_node,
            &[("vexpand", "true"), ("padding", "12")],
        );
        create_widget(&mut state, child, "ng-button");
        set_attrs(
            &mut state,
            child,
            &[("halign", "start"), ("width", "120"), ("height", "32")],
        );

        for (p, c) in [(win, box_node), (box_node, child)] {
            state.apply_command(
                &RenderCommand::AppendChild {
                    parent: p,
                    child: c,
                },
                "AnchorComment",
                |_| "NSView",
            );
        }

        let root_rects = solve_scene_layout(&state, win, 420, 320);
        assert_eq!(
            root_rects[&child],
            LayoutRect {
                x: 22,
                y: 22,
                width: 120,
                height: 32,
            }
        );

        let win32_bounds = compute_parent_relative_bounds(&state, &root_rects, false);
        assert_eq!(
            win32_bounds[&child],
            LayoutRect {
                x: 12,
                y: 12,
                width: 120,
                height: 32,
            }
        );

        let cocoa_bounds = compute_parent_relative_bounds(&state, &root_rects, true);
        // Parent box height is 300; child is 12pt from top and 32pt tall -> bottom-up y = 300 - 12 - 32 = 256.
        assert_eq!(
            cocoa_bounds[&child],
            LayoutRect {
                x: 12,
                y: 256,
                width: 120,
                height: 32,
            }
        );
    }

    #[test]
    fn preserves_first_style_class_spacing_when_later_class_leaves_zero_and_supports_gap_max_size()
    {
        let mut state = NativeSceneState::new();
        let root = nid(0);
        let box_node = nid(1);
        let a = nid(2);
        let b = nid(3);

        state.apply_command(
            &RenderCommand::RegisterStyle {
                style_id: StyleId(1),
                rule: StyleRule {
                    class_name: String::from("padded"),
                    padding: 16,
                    spacing: 10,
                    ..StyleRule::default()
                },
            },
            "AnchorComment",
            |_| "NSView",
        );
        state.apply_command(
            &RenderCommand::RegisterStyle {
                style_id: StyleId(2),
                rule: StyleRule {
                    class_name: String::from("accent"),
                    foreground: Some(String::from("#22c55e")),
                    ..StyleRule::default()
                },
            },
            "AnchorComment",
            |_| "NSView",
        );

        create_widget(&mut state, root, "ng-window");
        create_widget(&mut state, box_node, "ng-box");
        set_attrs(&mut state, box_node, &[("gap", "14"), ("max-size", "400")]);
        state.apply_command(
            &RenderCommand::ApplyStyleClass {
                node: box_node,
                style_id: StyleId(1),
            },
            "AnchorComment",
            |_| "NSView",
        );
        state.apply_command(
            &RenderCommand::ApplyStyleClass {
                node: box_node,
                style_id: StyleId(2),
            },
            "AnchorComment",
            |_| "NSView",
        );
        create_widget(&mut state, a, "ng-label");
        set_attrs(&mut state, a, &[("height", "20")]);
        create_widget(&mut state, b, "ng-label");
        set_attrs(&mut state, b, &[("height", "20")]);
        for (p, c) in [(root, box_node), (box_node, a), (box_node, b)] {
            state.apply_command(
                &RenderCommand::AppendChild {
                    parent: p,
                    child: c,
                },
                "AnchorComment",
                |_| "NSView",
            );
        }

        let rects = solve_scene_layout(&state, root, 900, 600);
        // Box has padding=16 (preserved from class 1), gap=14 overriding spacing, and max-size=400 capping inner width:
        // root inner x=12, w=876; box inner x = 12 + 16 + (844 - 400)/2 = 250, inner w = 400; child b y = 12 + 16 + 20 + 14 = 62.
        assert_eq!(rects[&a].x, 250);
        assert_eq!(rects[&a].width, 400);
        assert_eq!(rects[&b].y, 62);
    }
}
