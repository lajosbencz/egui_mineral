use crate::Material;
use crate::MaterialExt;
use egui_mineral::i18n::I18n;
use egui_mineral::semantic::KeyedSalt;
use egui_mineral::widget::MineralUi;
use std::sync::Arc;

use egui::{
    DroppedFileHandle, Galley, Layout, Pos2, Sense, Shape, Stroke,
    epaint::{RoundedRect, tessellator::path},
    vec2,
};
use egui_mineral::{
    layout::{Constraints, Measure},
    platform::FileFilter,
    semantic::{Role, SemanticNode, StableId},
    widget::{MineralWidget, WidgetFrame, WidgetResponse},
};

use crate::actions::{ButtonVariant, PushButton};
use crate::widget::{common, parts};
use crate::{
    spec::{
        ColorRole, TypeScale,
        defaults::dialog as d,
        tokens::{baseline_button, dialog, outlined_card, outlined_text_field},
    },
    tr,
};

// No MD3 counterpart

const PAD: f32 = d::PADDING;
const TITLE_TYPE: TypeScale = TypeScale::TitleMedium;
// 4dp grid
const DASH: [f32; 2] = [8.0, 4.0];

/// Events of [`FileDropZone`], read from `WidgetResponse::output`.
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct FileDropZoneOutput {
    /// Files dropped this frame that pass the filter and the limit.
    pub dropped: Vec<DroppedFileHandle>,
    /// Files dropped this frame that fail the filter or exceed the limit.
    pub rejected: Vec<DroppedFileHandle>,
    /// Files are dragged over the zone.
    pub hovering: bool,
    /// Browse button clicked; the app opens its file dialog.
    pub browse_clicked: bool,
}

/// Drop area and browse button for files; non-MD3 (Chakra `FileUpload` pattern).
#[derive(Clone, Debug)]
#[must_use = "show with ui.mineral(..)"]
pub struct FileDropZone {
    salt: KeyedSalt,
    title: Option<String>,
    supporting: Option<String>,
    filter: FileFilter,
    max_files: Option<usize>,
    disabled: bool,
}

impl FileDropZone {
    /// Accepts any number of any file; agent key prefix `<id_salt>`.
    pub fn new(id_salt: impl Into<KeyedSalt>) -> Self {
        let id_salt: KeyedSalt = id_salt.into();
        Self {
            salt: id_salt,
            title: None,
            supporting: None,
            filter: FileFilter::default(),
            max_files: None,
            disabled: false,
        }
    }

    /// Accepted names and types; default accepts all.
    pub fn accept(mut self, filter: FileFilter) -> Self {
        self.filter = filter;
        self
    }

    /// Accepted files per drop; extras go to [`FileDropZoneOutput::rejected`].
    pub fn max_files(mut self, max_files: usize) -> Self {
        self.max_files = Some(max_files);
        self
    }

    /// Headline; default the `mineral-file-drop` message.
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Hint below the title, e.g. accepted types.
    pub fn supporting(mut self, supporting: impl Into<String>) -> Self {
        self.supporting = Some(supporting.into());
        self
    }

    crate::widget::setters! {
        /// Greys out the zone; drops and browsing are ignored.
        disabled
    }

    crate::widget::setters! {
        /// Replaces the `<id_salt>` key prefix.
        agent_key(salt)
    }

    fn shown_title(&self, i18n: &egui_mineral::i18n::I18n) -> String {
        crate::tr_or(i18n, self.title.as_deref(), "mineral-file-drop")
    }

    fn accepts(&self, file: &DroppedFileHandle) -> bool {
        if self.filter.filters().is_empty() {
            return true;
        }
        let ext = file
            .path()
            .extension()
            .map_or_else(String::new, |e| e.to_string_lossy().to_lowercase());
        self.filter
            .filters()
            .iter()
            .flat_map(|t| &t.extensions)
            .any(|e| e.trim_start_matches('.').eq_ignore_ascii_case(&ext))
    }

    fn text(&self, ui: &MineralUi<'_, Material>, width: f32, disabled: bool) -> (Arc<Galley>, Option<Arc<Galley>>) {
        let rt = ui.rt();
        let t = rt.tokens();
        let c = |role: ColorRole| {
            let color = t.role(role);
            if disabled { common::disabled_content(&t) } else { color }
        };
        let p = ui.painter();
        let title = parts::type_wrapped(
            p,
            &self.shown_title(ui.i18n()),
            TITLE_TYPE,
            c(dialog::HEADLINE_COLOR),
            width,
            0,
        );
        let supporting = self.supporting.as_ref().map(|s| {
            parts::type_wrapped(
                p,
                s,
                dialog::SUPPORTING_TEXT_FONT,
                c(dialog::SUPPORTING_TEXT_COLOR),
                width,
                0,
            )
        });
        (title, supporting)
    }

    fn content_height(&self, ui: &MineralUi<'_, Material>, width: f32) -> f32 {
        let (title, supporting) = self.text(ui, width, self.disabled);
        dialog::ICON_SIZE
            + d::ICON_BOTTOM_PADDING
            + title.size().y
            + supporting.map_or(0.0, |g| d::TITLE_BOTTOM_PADDING + g.size().y)
            + d::TEXT_BOTTOM_PADDING
            + baseline_button::CONTAINER_HEIGHT
    }

    fn partition(&self, files: Vec<DroppedFileHandle>) -> (Vec<DroppedFileHandle>, Vec<DroppedFileHandle>) {
        let (mut dropped, mut rejected) = (Vec::new(), Vec::new());
        for f in files {
            if self.accepts(&f) && self.max_files.is_none_or(|m| dropped.len() < m) {
                dropped.push(f);
            } else {
                rejected.push(f);
            }
        }
        (dropped, rejected)
    }
}

impl MineralWidget for FileDropZone {
    type Kit = Material;
    type State = ();
    type Output = FileDropZoneOutput;

    egui_mineral::kit::widget_meta!(salt, enabled, no_target, hover);

    fn layout(&self, ui: &MineralUi<'_, Material>, c: Constraints, (): &Self::State) -> Measure {
        let content = self.content_height(ui, c.max.x - 2.0 * PAD);
        c.constrain(vec2(c.max.x, content + 2.0 * PAD)).into()
    }

    fn update(
        &mut self,
        ui: &mut MineralUi<'_, Material>,
        frame: &WidgetFrame<'_>,
        (): &mut (),
    ) -> WidgetResponse<FileDropZoneOutput> {
        let rt = ui.rt();
        let ctx = rt.ctx();
        let t = rt.tokens();
        let rect = frame.zones.visual;
        let over = |p: Option<Pos2>| p.is_none_or(|p| rect.contains(p));
        let (hovered_files, dropped_files, pointer) = ctx.input(|i| {
            (
                !i.raw.hovered_files.is_empty(),
                i.raw.dropped_files.clone(),
                i.pointer.latest_pos(),
            )
        });
        let hovering = !self.disabled && hovered_files && over(pointer);
        let (dropped, rejected) = if !self.disabled && over(pointer) {
            self.partition(dropped_files)
        } else {
            (Vec::new(), Vec::new())
        };

        let r = outlined_card::CONTAINER_SHAPE.corner_radius(&t.shape, rt.i18n().direction());
        let p = ui.painter();
        let accent = t.role(outlined_text_field::FOCUS_OUTLINE_COLOR);
        if hovering {
            p.rect_filled(rect, r, common::with_alpha(accent, t.state.hover));
        }
        let (w, col) = if hovering {
            (outlined_text_field::FOCUS_OUTLINE_WIDTH, accent)
        } else if self.disabled {
            (
                outlined_card::OUTLINE_WIDTH,
                t.role_alpha(
                    outlined_card::DISABLED_OUTLINE_COLOR,
                    outlined_card::DISABLED_OUTLINE_OPACITY,
                ),
            )
        } else {
            (outlined_card::OUTLINE_WIDTH, t.role(outlined_card::OUTLINE_COLOR))
        };
        let mut outline = Vec::new();
        path::rounded_rectangle(&mut outline, RoundedRect::new(rect.shrink(w / 2.0), r));
        if let Some(&first) = outline.first() {
            outline.push(first);
        }
        p.extend(Shape::dashed_line(&outline, Stroke::new(w, col), DASH[0], DASH[1]));

        let inner = rect.shrink(PAD);
        let (title, supporting) = self.text(ui, inner.width(), self.disabled);
        let content = self.content_height(ui, inner.width());
        let icon = if self.disabled {
            common::disabled_content(&t)
        } else if hovering {
            accent
        } else {
            t.role(dialog::ICON_COLOR)
        };
        let scope = self.salt.child("content");
        let browse = egui_mineral::layout::child(ui, scope, inner, Layout::top_down(egui::Align::Center), |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.add_space(((inner.height() - content) / 2.0).max(0.0));
            let (ir, _) = ui.allocate_exact_size(vec2(dialog::ICON_SIZE, dialog::ICON_SIZE), Sense::hover());
            common::icon(
                ui.painter(),
                ir.center(),
                if hovering {
                    ui.icons().cloud_upload
                } else {
                    ui.icons().upload_file
                },
                dialog::ICON_SIZE,
                icon,
            );
            ui.add_space(d::ICON_BOTTOM_PADDING);
            ui.add(egui::Label::new(title));
            if let Some(g) = supporting {
                ui.add_space(d::TITLE_BOTTOM_PADDING);
                ui.add(egui::Label::new(g));
            }
            ui.add_space(d::TEXT_BOTTOM_PADDING);
            ui.mineral(
                PushButton::new(self.salt.child("browse"), tr(rt.i18n(), "mineral-file-browse"))
                    .variant(ButtonVariant::Tonal)
                    .icon(ui.icons().attach_file)
                    .disabled(self.disabled)
                    .agent_key(StableId::of(&self.salt, "browse-button")),
            )
            .clicked()
        })
        .inner;

        let changed = browse || !dropped.is_empty();
        WidgetResponse::new(
            frame.response.clone(),
            FileDropZoneOutput {
                dropped,
                rejected,
                hovering,
                browse_clicked: browse,
            },
        )
        .with_changed(changed)
    }

    fn semantics(&self, i18n: &I18n, _: &WidgetResponse<FileDropZoneOutput>, (): &()) -> Option<SemanticNode> {
        let title = self.shown_title(i18n);
        let mut n = SemanticNode::new(StableId::of(&self.salt, "dropzone"), Role::Group, &title);
        n.state.disabled = self.disabled;
        Some(n)
    }
}
