//! The sidebar down the window's left: the logo, the launcher's pages (Installed, Store,
//! Updates and, while there are any, Hidden Apps), Settings, and at its foot the last check and
//! the community links. It folds down to its icons with the toggle in the title bar, sliding
//! between the two widths.

use egui::{Color32, CornerRadius, FontId, Id, Rect, Sense, Stroke, Ui, UiBuilder, emath::easing, lerp, pos2, vec2};

use crate::app::{Dialog, LauncherUi, TitleBar};
use crate::i18n::fmt;
use crate::settings::Filter;
use crate::theme::{self, Tokens, tint};
use crate::widgets;

/// The sidebar's width, open.
pub(crate) const SIDEBAR_W: f32 = 224.0;
/// The sidebar's width, folded down to its icons.
pub(crate) const SIDEBAR_COLLAPSED_W: f32 = 64.0;
/// How long the sidebar takes to fold or open, in seconds (none when the OS asks for less motion).
const COLLAPSE_TIME: f32 = 0.22;
/// Room round the sidebar's contents.
const PAD: f32 = 12.0;
/// Room macOS's traffic lights take at the left of the title bar's row. A sidebar narrower than
/// this leaves the rest of it to the title bar.
pub(crate) const TRAFFIC_LIGHTS_W: f32 = 76.0;
/// A page entry's height.
const ITEM_H: f32 = 36.0;
/// The social logos at the foot, a little smaller than the old footer's so all eight fit.
const LINK_ICON: f32 = 11.0;

/// The drawn icon in front of a sidebar entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NavIcon {
    Installed,
    Store,
    Updates,
    Hidden,
    Settings,
}

/// One page in the sidebar: the filter it shows, its label and the count beside it.
struct Page {
    filter: Filter,
    label: &'static str,
    count: Option<usize>,
    icon: NavIcon,
}

/// The title of the page that shows `filter`, as the sidebar and the page heading name it.
pub(crate) fn page_title(filter: Filter) -> &'static str {
    match filter {
        Filter::Installed => tl!("Installed"),
        Filter::Updates => tl!("Updates"),
        Filter::Hidden => tl!("Hidden Apps"),
        Filter::All | Filter::NotInstalled => tl!("Store"),
    }
}

/// Fades `color` by `alpha` (0: gone, 1: as it is).
fn faded(color: Color32, alpha: f32) -> Color32 {
    color.gamma_multiply(alpha.clamp(0.0, 1.0))
}

impl LauncherUi {
    /// How far the sidebar is folded this frame: 0 open, 1 down to its icons, eased in between
    /// while it moves. Instant when the OS asks for less motion.
    pub(crate) fn sidebar_folded(&self, ctx: &egui::Context) -> f32 {
        let time = if self.reduce_motion { 0.0 } else { COLLAPSE_TIME };
        ctx.animate_bool_with_time_and_easing(Id::unique("sidebar-folded"), self.settings.sidebar_collapsed, time, easing::cubic_in_out)
    }

    /// The pages this build offers. A build that can't manage apps (the web) only has the Store.
    fn pages(&self) -> Vec<Page> {
        let mut pages = Vec::new();
        if self.caps.manage_apps {
            let installed = self.count(Filter::Installed);
            pages.push(Page { filter: Filter::Installed, label: tl!("Installed"), count: Some(installed), icon: NavIcon::Installed });
        }
        pages.push(Page { filter: Filter::All, label: tl!("Store"), count: None, icon: NavIcon::Store });
        if self.caps.manage_apps {
            let updates = self.count(Filter::Updates);
            pages.push(Page { filter: Filter::Updates, label: tl!("Updates"), count: (updates > 0).then_some(updates), icon: NavIcon::Updates });
        }
        // Hiding an app is only undone from here, so the way back stays while any are hidden.
        if !self.settings.hidden.is_empty() {
            pages.push(Page { filter: Filter::Hidden, label: tl!("Hidden Apps"), count: Some(self.settings.hidden.len()), icon: NavIcon::Hidden });
        }
        pages
    }

    /// Keeps the saved page one this build can show: the web has no Installed or Updates page,
    /// and Hidden Apps goes once nothing is hidden.
    pub(crate) fn settle_page(&mut self) {
        let f = self.settings.filter;
        let gone = match f {
            Filter::Installed | Filter::Updates => !self.caps.manage_apps,
            Filter::Hidden => self.settings.hidden.is_empty(),
            // Not installed has no tab of its own (it shows as the Store), but a caller may
            // still narrow the Store to it.
            Filter::NotInstalled | Filter::All => false,
        };
        if gone {
            self.settings.filter = Filter::All;
        }
    }

    /// Draws the sidebar, `folded` of the way down to its icons ([`Self::sidebar_folded`]). Its
    /// top strip is the title bar's row: the logo sits centred on the same line as the title, and
    /// on macOS just right of the traffic lights, which sit over the sidebar's top-left corner.
    pub(crate) fn sidebar(&mut self, ui: &mut Ui, bar: TitleBar, folded: f32) {
        let t = Tokens::get(ui.ctx());
        let full = ui.max_rect();
        // Everything is clipped to the sidebar as it narrows.
        ui.set_clip_rect(ui.clip_rect().intersect(full));
        // The top: a title-bar strip (draggable with a custom or macOS title bar) with the logo.
        let top = Rect::from_min_size(full.min, vec2(full.width(), crate::views::HEADER_H));
        if bar != TitleBar::Native {
            ui.scope_builder(UiBuilder::new().max_rect(top), widgets::title_bar_drag);
        }
        // The wordmark fades out over the first half of the fold and the "A" mark in over the
        // second, so the two never overlap. On macOS the traffic lights take that corner folded.
        let open = 1.0 - folded;
        let wordmark = (1.0 - folded * 2.0).clamp(0.0, 1.0);
        let mark = (folded * 2.0 - 1.0).clamp(0.0, 1.0);
        if wordmark > 0.0 {
            // A touch smaller beside the traffic lights, so it stays clear of the content panel.
            let logo_h = if bar == TitleBar::MacOs { 12.0 } else { 14.0 };
            let logo = vec2(logo_h * crate::icons::ARTCRAFT_LOGO_ASPECT, logo_h);
            let left = if bar == TitleBar::MacOs { TRAFFIC_LIGHTS_W } else { PAD + 10.0 };
            let logo_rect = Rect::from_min_size(pos2(top.left() + left, top.center().y - logo_h / 2.0), logo);
            egui::Image::new(crate::icons::artcraft_logo(t.dark))
                .alt_text(tl!("ArtCraft"))
                .tint(faded(Color32::WHITE, wordmark))
                .paint_at(ui, logo_rect);
        }
        if mark > 0.0 && bar != TitleBar::MacOs {
            let rect = Rect::from_center_size(pos2(full.left() + SIDEBAR_COLLAPSED_W / 2.0, top.center().y), vec2(22.0, 22.0));
            egui::Image::new(crate::icons::artcraft_mark()).tint(faded(Color32::WHITE, mark)).paint_at(ui, rect);
        }

        let body = Rect::from_min_max(pos2(full.left() + PAD, top.bottom() + 4.0), pos2(full.right() - PAD, full.bottom() - PAD));
        ui.scope_builder(UiBuilder::new().max_rect(body), |ui| {
            ui.spacing_mut().item_spacing.y = 2.0;
            for page in self.pages() {
                let selected = self.settings.filter == page.filter;
                let item = Item { icon: page.icon, label: page.label, count: page.count, selected, news: page.filter == Filter::Updates };
                if nav_item(ui, &t, &item, folded).clicked() {
                    self.settings.filter = page.filter;
                }
            }
        });

        // The foot: the last check and the community links, laid out at the open width so they
        // don't reflow as the sidebar moves, and fading out as it folds. Settings sits above them,
        // and slides down to the bottom as they go.
        let inner_w = SIDEBAR_W - 2.0 * PAD;
        let foot_top = if open > 0.0 {
            let foot = Rect::from_min_max(pos2(full.left() + PAD, top.bottom()), pos2(full.left() + PAD + inner_w, full.bottom() - PAD));
            let shown = ui.scope_builder(UiBuilder::new().max_rect(foot).layout(egui::Layout::bottom_up(egui::Align::Min)), |ui| {
                ui.set_opacity(open);
                // Half folded, they are only fading away: nothing there to click.
                if folded > 0.5 {
                    ui.disable();
                }
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.horizontal(|ui| widgets::icon_links(ui, LINK_ICON, &self.community_links()));
                ui.add_space(6.0);
                self.check_status(ui, &t);
                ui.add_space(12.0);
                let (rect, _) = ui.allocate_exact_size(vec2(ui.available_width(), 1.0), Sense::hover());
                ui.painter().hline(rect.x_range(), rect.center().y, t.hairline());
                ui.add_space(8.0);
            });
            shown.response.rect.top()
        } else {
            full.bottom() - PAD
        };
        let settings_bottom = lerp(foot_top..=full.bottom() - PAD, folded);
        let settings = Rect::from_min_max(pos2(full.left() + PAD, settings_bottom - ITEM_H), pos2(full.right() - PAD, settings_bottom));
        ui.scope_builder(UiBuilder::new().max_rect(settings), |ui| {
            let open = matches!(self.dialog, Some(Dialog::Settings));
            let item = Item { icon: NavIcon::Settings, label: tl!("Settings"), count: None, selected: open, news: false };
            if nav_item(ui, &t, &item, folded).clicked() {
                self.dialog = Some(Dialog::Settings);
            }
        });
    }

    /// When the catalog was last checked, and from where; while GitHub is rate limited, a warning
    /// under it. Both wrap to the sidebar's width.
    fn check_status(&mut self, ui: &mut Ui, t: &Tokens) {
        let (status, notice) = self.status_texts();
        let line = |ui: &mut Ui, s: &str, color: Color32| {
            egui::Frame::new()
                .inner_margin(egui::Margin { left: 4, right: 4, top: 0, bottom: 0 })
                .show(ui, |ui| ui.add(egui::Label::new(egui::RichText::new(s).size(11.5).color(color)).wrap()))
                .inner
        };
        // Bottom up: the notice goes in first so that it sits under the status.
        if let Some((full, short)) = notice {
            line(ui, &short, t.warn).on_hover_text(full);
            ui.add_space(4.0);
        }
        line(ui, &status, t.faint);
    }

    /// The website, GitHub and Discord, then ArtCraft's social accounts, in reading order.
    pub(crate) fn community_links(&self) -> Vec<widgets::IconLink<'static>> {
        use artcraft_launcher_core::catalog::{GITHUB_URL, SOCIAL, WEBSITE_URL};
        let named =
            |name: &'static str, label: &str, url: &'static str| widgets::IconLink { name, label: label.to_string(), tip: label.to_string(), url };
        let mut links = vec![
            named("Website", tl!("Website"), WEBSITE_URL),
            named("GitHub", tl!("GitHub"), GITHUB_URL),
            widgets::IconLink {
                name: "Discord",
                label: tl!("Discord").to_string(),
                tip: tl!("Join the ArtCraft community on Discord").to_string(),
                url: self.discord_url,
            },
        ];
        links.extend(SOCIAL.iter().map(|(name, url)| widgets::IconLink {
            name,
            label: (*name).to_string(),
            tip: fmt(tl!("ArtCraft on {name}"), &[("name", name)]),
            url,
        }));
        links
    }
}

/// What one sidebar entry shows.
struct Item<'a> {
    icon: NavIcon,
    label: &'a str,
    /// The number beside the label (apps installed, updates waiting, apps hidden).
    count: Option<usize>,
    selected: bool,
    /// The count is news (updates waiting): a filled accent pill, and a badge on the icon when
    /// folded, instead of a quiet number.
    news: bool,
}

/// One sidebar entry: an icon, its label and an optional count, the whole row clickable. The
/// selected entry takes the content panel's colour, so it reads as the page on show. As the
/// sidebar folds (`folded` towards 1) the label and count fade out and the icon moves to the
/// middle; folded, the label is the entry's tooltip and news is a badge on the icon.
fn nav_item(ui: &mut Ui, t: &Tokens, item: &Item<'_>, folded: f32) -> egui::Response {
    let Item { icon, label, count, selected, news } = *item;
    let (rect, resp) = ui.allocate_exact_size(vec2(ui.available_width(), ITEM_H), Sense::click());
    let hovered = resp.hovered();
    let p = ui.painter();
    let radius = CornerRadius::same(8);
    if selected {
        p.rect_filled(rect, radius, t.bg);
        p.rect_stroke(rect, radius, t.hairline(), egui::StrokeKind::Inside);
        // A short accent bar at the left edge marks the page on show.
        let bar = Rect::from_center_size(pos2(rect.left() + 1.5, rect.center().y), vec2(3.0, 16.0));
        p.rect_filled(bar, CornerRadius::same(2), t.accent);
    } else if hovered {
        p.rect_filled(rect, radius, tint(t.ink, if t.dark { 0.06 } else { 0.05 }));
    }
    let ink = if selected || hovered { t.ink_strong } else { t.muted };
    // In front of the label when open; in the middle of the row once the row is too narrow.
    let icon_rect = Rect::from_center_size(pos2((rect.left() + 22.0).min(rect.center().x), rect.center().y), vec2(16.0, 16.0));
    paint_icon(p, icon_rect, icon, ink);
    // The label and count are gone a little before the sidebar is fully folded.
    let text_alpha = (1.0 - folded * 1.6).clamp(0.0, 1.0);
    if text_alpha > 0.0 {
        let text_left = rect.left() + 40.0;
        let mut right = rect.right() - 10.0;
        if let Some(n) = count {
            let text = n.to_string();
            if news {
                // News (updates waiting) is a filled accent pill; plain counts stay quiet.
                let galley = p.layout_no_wrap(text, FontId::new(11.0, theme::mono_bold()), Color32::WHITE);
                let pill = Rect::from_min_size(pos2(right - galley.size().x - 12.0, rect.center().y - 9.0), vec2(galley.size().x + 12.0, 18.0));
                p.rect_filled(pill, CornerRadius::same(9), faded(t.accent, text_alpha));
                p.galley(pill.center() - galley.size() / 2.0, galley, faded(Color32::WHITE, text_alpha));
                right = pill.left() - 6.0;
            } else {
                let galley = p.layout_no_wrap(text, FontId::new(11.0, theme::mono()), t.faint);
                let pos = pos2(right - galley.size().x, rect.center().y - galley.size().y / 2.0);
                right = pos.x - 6.0;
                p.galley(pos, galley, faded(t.faint, text_alpha));
            }
        }
        let mut job = egui::text::LayoutJob::single_section(
            label.to_string(),
            egui::TextFormat {
                font_id: FontId::new(13.5, if selected { theme::semibold() } else { theme::medium() }),
                color: faded(ink, text_alpha),
                ..Default::default()
            },
        );
        // Open, a long label is cut short with "…". While the sidebar moves it is left whole and
        // clipped to the row, so it slides under the edge instead of changing as it goes.
        if folded == 0.0 {
            job.wrap = egui::text::TextWrapping::truncate_at_width((right - text_left).max(0.0));
        }
        let galley = p.layout_job(job);
        p.with_clip_rect(rect).galley(pos2(text_left, rect.center().y - galley.size().y / 2.0), galley, faded(ink, text_alpha));
    }
    // Folded, updates waiting show as a small badge on the icon's corner.
    let badge_alpha = (folded * 2.0 - 1.0).clamp(0.0, 1.0);
    if news && badge_alpha > 0.0 && count.is_some_and(|n| n > 0) {
        let text = count.map_or_else(String::new, |n| if n > 9 { "9+".to_string() } else { n.to_string() });
        let galley = p.layout_no_wrap(text, FontId::new(9.0, theme::mono_bold()), Color32::WHITE);
        let size = vec2((galley.size().x + 7.0).max(14.0), 14.0);
        let badge = Rect::from_center_size(icon_rect.right_top() + vec2(2.0, 1.0), size);
        p.rect_filled(badge, CornerRadius::same(7), faded(t.accent, badge_alpha));
        p.rect_stroke(badge, CornerRadius::same(7), Stroke::new(1.5, faded(t.chrome, badge_alpha)), egui::StrokeKind::Outside);
        p.galley(badge.center() - galley.size() / 2.0, galley, faded(Color32::WHITE, badge_alpha));
    }
    let name = match count {
        Some(n) => format!("{label} {n}"),
        None => label.to_string(),
    };
    resp.widget_info(|| egui::WidgetInfo::selected(egui::Role::Tab, true, selected, &name));
    // Folded, the label is no longer on screen: it shows on hover instead.
    let resp = if folded > 0.5 { resp.on_hover_text(&name) } else { resp };
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// The title bar's button that folds the sidebar down to its icons and opens it again. Its icon
/// is a window with a side pane, filled in while the sidebar is open. It takes the bar's full
/// `height`, with the button centred in it like the title beside it.
pub(crate) fn sidebar_toggle(ui: &mut Ui, t: &Tokens, collapsed: bool, height: f32) -> egui::Response {
    let label = if collapsed { tl!("Expand sidebar") } else { tl!("Collapse sidebar") };
    let (cell, _) = ui.allocate_exact_size(vec2(30.0, height), Sense::hover());
    let rect = Rect::from_center_size(cell.center(), vec2(30.0, 30.0));
    let resp = ui.interact(rect, ui.scope_id().with("sidebar-toggle"), Sense::click());
    let hovered = resp.hovered();
    let p = ui.painter();
    if hovered {
        p.rect_filled(rect, theme::control_r(), tint(t.ink, if t.dark { 0.08 } else { 0.06 }));
    }
    let ink = if hovered { t.ink_strong } else { t.muted };
    let frame = Rect::from_center_size(rect.center(), vec2(16.0, 13.0));
    let pane = Rect::from_min_max(frame.min, pos2(frame.left() + 5.5, frame.bottom()));
    if !collapsed {
        p.rect_filled(pane, CornerRadius { nw: 3, sw: 3, ne: 0, se: 0 }, faded(ink, 0.45));
    }
    p.rect_stroke(frame, CornerRadius::same(3), Stroke::new(1.3, ink), egui::StrokeKind::Middle);
    p.vline(pane.right(), frame.y_range(), Stroke::new(1.3, ink));
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, label));
    resp.on_hover_text(label).on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// The small line icons in front of the sidebar's entries, drawn into `rect` (16 × 16).
fn paint_icon(p: &egui::Painter, rect: Rect, icon: NavIcon, color: Color32) {
    let s = Stroke::new(1.4, color);
    let c = rect.center();
    match icon {
        NavIcon::Installed => {
            // Four tiles: the apps on this computer.
            for (dx, dy) in [(-1.0, -1.0), (1.0, -1.0), (-1.0, 1.0), (1.0, 1.0)] {
                let tile = Rect::from_center_size(c + vec2(dx * 3.75, dy * 3.75), vec2(5.5, 5.5));
                p.rect_stroke(tile, CornerRadius::same(1), s, egui::StrokeKind::Middle);
            }
        }
        NavIcon::Store => {
            // A shopping bag.
            let bag = Rect::from_min_max(c + vec2(-6.0, -2.5), c + vec2(6.0, 7.0));
            p.rect_stroke(bag, CornerRadius::same(2), s, egui::StrokeKind::Middle);
            let handle: Vec<_> = (0..=12)
                .map(|i| {
                    let a = std::f32::consts::PI * (1.0 + i as f32 / 12.0);
                    c + vec2(3.0 * a.cos(), -2.5 + 4.0 * a.sin())
                })
                .collect();
            p.line(handle, s);
        }
        NavIcon::Updates => {
            // An arrow down onto a tray.
            p.line_segment([c + vec2(0.0, -7.0), c + vec2(0.0, 3.0)], s);
            p.line(vec![c + vec2(-3.5, -0.5), c + vec2(0.0, 3.0), c + vec2(3.5, -0.5)], s);
            p.line(vec![c + vec2(-6.5, 2.5), c + vec2(-6.5, 6.5), c + vec2(6.5, 6.5), c + vec2(6.5, 2.5)], s);
        }
        NavIcon::Hidden => {
            // An eye, struck through.
            let top: Vec<_> =
                (0..=16).map(|i| c + vec2(-7.0 + 14.0 * i as f32 / 16.0, -4.0 * (std::f32::consts::PI * i as f32 / 16.0).sin())).collect();
            let bottom: Vec<_> = top.iter().map(|q| pos2(q.x, 2.0 * c.y - q.y)).collect();
            p.line(top, s);
            p.line(bottom, s);
            p.circle_stroke(c, 1.8, s);
            p.line_segment([c + vec2(-6.0, 6.0), c + vec2(6.0, -6.0)], s);
        }
        NavIcon::Settings => widgets::paint_glyph(p, rect.expand(4.0), widgets::Glyph::Settings, color),
    }
}
