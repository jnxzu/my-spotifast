//! The left panel: navigation and Your Library.

use egui::{CornerRadius, Frame, Margin, Rect, Sense, Vec2, pos2, vec2};

use crate::api::models::pick_image;
use crate::app::App;
use crate::i18n::{Locale, gettext};
use crate::model::{Action, DragEntry, DragTrack, Page};
use crate::settings::{LIKED_SONGS_KEY, LibraryShelf as Filter, LibrarySort};
use crate::theme::{self, Icon, Palette};

const DEFAULT_ROW_HEIGHT: f32 = 60.0;
const COMPACT_ROW_HEIGHT: f32 = 32.0;

struct Entry {
    image: Option<String>,
    grid_image: Option<String>,
    name: String,
    subtitle: String,
    grid_subtitle: String,
    page: Page,
    uri: String,
    round: bool,
    liked: bool,
    /// The account's own playlist: the one it may rename and delete.
    owned: bool,
    /// A playlist the account may drop songs on.
    editable: bool,
    playlist_index: Option<usize>,
    /// A folder row: its rootlist id, whether it is rolled up, and how
    /// many playlists it holds.
    folder: Option<(String, bool, usize)>,
    /// How deep inside folders the row sits, for the indent.
    depth: u8,
}

impl Entry {
    fn ordering_key(&self) -> &str {
        if self.liked {
            LIKED_SONGS_KEY
        } else {
            &self.uri
        }
    }
}

/// The context a library row plays, matching the cover play button and the
/// right-click menu. Liked Songs has no URI of its own and plays the
/// account's collection instead.
fn entry_play_uri(app: &App, entry: &Entry) -> Option<String> {
    if entry.folder.is_some() || entry.page == Page::Episodes {
        return None;
    }
    if entry.liked {
        app.user
            .as_ref()
            .map(|user| format!("spotify:user:{}:collection", user.id))
    } else if entry.uri.is_empty() {
        None
    } else {
        Some(entry.uri.clone())
    }
}

fn entry_is_playing_context(entry: &Entry, context: Option<&str>) -> bool {
    if entry.liked {
        context.is_some_and(|context| context.ends_with(":collection"))
    } else {
        !entry.uri.is_empty() && context == Some(entry.uri.as_str())
    }
}

fn cover_play_button(
    app: &mut App,
    ui: &mut egui::Ui,
    entry: &Entry,
    index: usize,
    cover_rect: Rect,
    parent: &egui::Response,
    current_context: bool,
) -> bool {
    let Some(uri) = entry_play_uri(app, entry) else {
        return false;
    };
    // The control pauses only while this context is actually playing; a
    // paused one keeps the play glyph, but pressing it resumes in place
    // instead of restarting the context.
    let playing = current_context && app.believed_playing();
    let play = ui.interact(
        cover_rect,
        ui.id().with(("library-cover-play", index)),
        Sense::click(),
    );
    let play_hover = play.hovered();
    if play_hover || parent.hovered() {
        let radius = if entry.round {
            (cover_rect.width() / 2.0).min(127.0) as u8
        } else {
            6
        };
        ui.painter().rect_filled(
            cover_rect,
            CornerRadius::same(radius),
            egui::Color32::from_black_alpha(120),
        );
        let icon = if playing {
            Icon::PauseFilled
        } else {
            Icon::PlayFilled
        };
        let icon_size = (cover_rect.width() * 0.24).clamp(18.0, 26.0);
        icon.image(
            if play_hover {
                app.palette.accent
            } else {
                egui::Color32::WHITE
            },
            icon_size,
        )
        .paint_at(
            ui,
            Rect::from_center_size(
                cover_rect.center() + theme::play_glyph_offset(icon, icon_size),
                Vec2::splat(icon_size),
            ),
        );
    }
    if !play.clicked() {
        return false;
    }
    // The first click of a double click plays; the second must not play again.
    if !play.double_clicked() {
        if current_context {
            app.actions.push(Action::TogglePlay);
        } else {
            app.actions.push(Action::PlayContext {
                uri,
                offset_uri: None,
                offset_index: None,
            });
        }
    }
    true
}

fn grid_play_rect(cover_rect: Rect) -> Rect {
    let size = (cover_rect.width() * 0.3).clamp(32.0, 44.0);
    let inset = LIBRARY_ITEM_PADDING + size / 2.0;
    Rect::from_center_size(
        pos2(cover_rect.right() - inset, cover_rect.bottom() - inset),
        Vec2::splat(size),
    )
}

fn grid_pin_rect(cover_rect: Rect) -> Rect {
    Rect::from_center_size(cover_rect.left_top() + Vec2::splat(12.0), Vec2::splat(20.0))
}

fn grid_play_button(
    app: &mut App,
    ui: &mut egui::Ui,
    entry: &Entry,
    cover_rect: Rect,
    parent: &egui::Response,
    playing_here: bool,
) -> bool {
    let Some(uri) = entry_play_uri(app, entry) else {
        return false;
    };
    let rect = grid_play_rect(cover_rect);
    let size = rect.width();
    let button = ui.interact(rect, ui.id().with("library-grid-play"), Sense::click());
    let playing = playing_here && app.believed_playing();
    let label = if playing {
        // Translators: The play button on a Library card. {name} is the playlist, album, artist, or podcast.
        gettext(app.locale, "Pause {name}")
    } else {
        // Translators: The play button on a Library card. {name} is the playlist, album, artist, or podcast.
        gettext(app.locale, "Play {name}")
    }
    .replace("{name}", &entry.name);
    button.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label.clone())
    });
    if parent.hovered() || playing_here || button.has_focus() {
        let fill = if button.hovered() {
            app.palette.accent_hover
        } else {
            app.palette.accent
        };
        ui.painter().add(
            egui::epaint::Shadow {
                offset: [0, 4],
                blur: 12,
                spread: 0,
                color: egui::Color32::from_black_alpha(90),
            }
            .as_shape(rect, egui::CornerRadius::same(127)),
        );
        ui.painter().circle_filled(rect.center(), size / 2.0, fill);
        let icon = if playing {
            Icon::PauseFilled
        } else {
            Icon::PlayFilled
        };
        let icon_size = size * 0.42;
        icon.image(app.palette.on_accent, icon_size).paint_at(
            ui,
            Rect::from_center_size(
                rect.center() + theme::play_glyph_offset(icon, icon_size),
                Vec2::splat(icon_size),
            ),
        );
    }
    if !button.clicked() {
        return false;
    }
    if playing_here {
        app.actions.push(Action::TogglePlay);
    } else {
        app.actions.push(Action::PlayContext {
            uri,
            offset_uri: None,
            offset_index: None,
        });
    }
    true
}

fn finish_entry_interaction(
    app: &mut App,
    ui: &mut egui::Ui,
    response: &egui::Response,
    entry: &Entry,
    cover_took_click: bool,
    custom_order: bool,
    drop_allowed: bool,
) {
    if response.hovered()
        && let Some(image) = &entry.image
    {
        app.actions.push(Action::PrepareTint(image.clone()));
    }
    if !entry.ordering_key().is_empty() && response.drag_started_by(egui::PointerButton::Primary) {
        egui::DragAndDrop::set_payload(
            ui.ctx(),
            DragEntry {
                uri: entry.ordering_key().to_string(),
                title: entry.name.clone(),
                image: entry.image.clone(),
            },
        );
    }
    if drop_allowed
        && (entry.liked || entry.editable)
        && egui::DragAndDrop::has_payload_of_type::<DragTrack>(ui.ctx())
        && let Some(track) = response.dnd_release_payload::<DragTrack>()
    {
        if entry.liked {
            app.actions.push(Action::SetSavedMany {
                uris: track
                    .items
                    .iter()
                    .map(|item| item.uri().to_string())
                    .collect(),
                saved: true,
            });
        } else if let Page::Playlist(id) = &entry.page {
            app.actions.push(Action::AddToPlaylist {
                playlist_id: id.clone(),
                playlist_name: entry.name.clone(),
                items: track.items.clone(),
            });
        }
    }
    theme::focus_ring(ui, response);
    if response.clicked() && !cover_took_click {
        if let Some((folder_id, _, _)) = &entry.folder {
            app.actions
                .push(Action::ToggleLibraryFolder(folder_id.clone()));
        } else {
            app.actions.push(Action::Open(entry.page.clone()));
        }
    }
    if !app.settings.sidebar_grid
        && response.double_clicked()
        && entry.folder.is_none()
        && !cover_took_click
        && let Some(uri) = entry_play_uri(app, entry)
    {
        app.actions.push(Action::PlayContext {
            uri,
            offset_uri: None,
            offset_index: None,
        });
    }
    entry_menu(app, response, entry, custom_order);
    crate::autoscroll::row(ui, response);
}

fn liked_entry(app: &App) -> Entry {
    let subtitle = match app.library.liked.total {
        Some(total) => app.locale.liked_song_count(total),
        None => gettext(app.locale, "Playlist").into_owned(),
    };
    let grid_subtitle = app
        .library
        .liked
        .total
        .map_or_else(String::new, |total| app.locale.song_count(total));
    Entry {
        image: None,
        grid_image: None,
        name: gettext(app.locale, "Liked Songs").into_owned(),
        subtitle,
        grid_subtitle,
        page: Page::LikedSongs,
        uri: String::new(),
        round: false,
        liked: true,
        owned: false,
        editable: false,
        playlist_index: None,
        folder: None,
        depth: 0,
    }
}

pub(crate) fn selected_sort(app: &App, shelf: Filter) -> LibrarySort {
    if let Some(sort) = app.settings.library_sort.get(&shelf).copied()
        && sort.supports(shelf)
    {
        return sort;
    }
    if shelf != Filter::Playlists {
        LibrarySort::Library
    } else if !app.settings.sidebar_order.is_empty() {
        LibrarySort::Local
    } else if app
        .rootlist
        .iter()
        .any(|row| matches!(row, crate::player::RootlistEntry::FolderStart { .. }))
    {
        LibrarySort::Spotify
    } else {
        LibrarySort::RecentlyPlayed
    }
}

pub fn show(app: &mut App, ui: &mut egui::Ui) {
    let palette = app.palette;
    let expanded_art = has_expanded_art(app);
    let floating_art = app.settings.sidebar_grid && expanded_art;
    // The traffic lights float over the top-left of the sidebar now, so the
    // first nav row has to start below them.
    let top = 12
        + theme::titlebar_inset(ui.ctx()) as i8
        + super::linux_left_controls_inset(ui.ctx()) as i8;
    let beside = if app.show_queue_panel || app.show_lyrics_panel {
        theme::SIDE_PANEL_MIN_WIDTH
    } else {
        0.0
    };
    let fit = super::yielding_panel(
        ui.ctx(),
        "sidebar",
        super::SIDEBAR_MIN_WIDTH..=600.0,
        app.settings.sidebar_width,
        ui.available_width() - super::topbar::least_width(ui.ctx()) - beside,
    );
    let panel = egui::Panel::left("sidebar")
        .resizable(true)
        .default_size(app.settings.sidebar_width)
        .size_range(fit.range.clone())
        .show_separator_line(false)
        .frame(Frame::new().fill(palette.panel).inner_margin(Margin {
            left: 12,
            right: 8,
            top,
            bottom: if expanded_art { 0 } else { 8 },
        }));
    let response = panel.show(ui, |ui| {
        let art_rect = expanded_art.then(|| expanded_art_rect(ui));
        if let Some(rect) = art_rect.filter(|_| !floating_art) {
            reserve_expanded_art(ui, rect);
        }
        contents(app, ui, art_rect.filter(|_| floating_art));
        if let Some(rect) = art_rect {
            if floating_art {
                paint_grid_art_mask(app, ui, rect);
            }
            paint_expanded_art(app, ui, rect);
        }
    });
    let width = response.response.rect.width();
    if (width - app.settings.sidebar_width).abs() > 1.0
        && super::panel_width_chosen(ui.ctx(), "sidebar", &fit)
    {
        app.settings.sidebar_width = width;
        app.actions.push(Action::SettingsChanged);
    }
}

fn expanded_art_rect(ui: &egui::Ui) -> Rect {
    let side = expanded_art_side(ui);
    Rect::from_min_size(
        pos2(
            ui.max_rect().left(),
            ui.max_rect().bottom() - EXPANDED_ART_GAP - side,
        ),
        Vec2::splat(side),
    )
}

fn reserve_expanded_art(ui: &mut egui::Ui, rect: Rect) {
    egui::Panel::bottom("sidebar-art-space")
        .exact_size(rect.height() + EXPANDED_ART_GAP)
        .resizable(false)
        .show_separator_line(false)
        .frame(Frame::new())
        .show(ui, |_| {});
}

/// In grid mode content continues behind the shared fixed artwork.
fn paint_grid_art_mask(app: &App, ui: &egui::Ui, rect: Rect) {
    let bottom_fade_rect = Rect::from_min_max(
        pos2(ui.max_rect().left(), rect.bottom() - 64.0),
        pos2(ui.max_rect().right(), rect.bottom()),
    );
    super::widgets::paint_vertical_gradient(
        ui,
        bottom_fade_rect,
        egui::Color32::TRANSPARENT,
        app.palette.panel,
    );
    ui.painter().rect_filled(
        Rect::from_min_max(
            pos2(ui.max_rect().left(), rect.bottom()),
            ui.max_rect().right_bottom(),
        ),
        0.0,
        app.palette.panel,
    );
}

fn has_expanded_art(app: &App) -> bool {
    app.settings.art_expanded
        && app
            .now_playing()
            .is_some_and(|now| now.art_url.is_some() || now.art_small.is_some())
}

fn expanded_art_side(ui: &egui::Ui) -> f32 {
    ui.max_rect()
        .width()
        .min(ui.max_rect().height() * 0.45)
        .max(80.0)
}

fn paint_expanded_art(app: &mut App, ui: &mut egui::Ui, rect: Rect) {
    let Some(now) = app.now_playing() else {
        return;
    };
    let Some(url) = now.art_url.clone().or_else(|| now.art_small.clone()) else {
        return;
    };
    let album_id = now.album_id.clone();
    let show_id = now.show_id.clone();
    let palette = app.palette;
    ui.painter().add(
        egui::epaint::Shadow {
            offset: [0, 0],
            blur: 28,
            spread: 0,
            color: egui::Color32::from_black_alpha(if palette.dark { 120 } else { 40 }),
        }
        .as_shape(rect, egui::CornerRadius::same(8)),
    );
    super::widgets::playing_cover(
        ui,
        &palette,
        app.backend.art(),
        "sidebar-art",
        Some(&url),
        rect,
        8.0,
    );
    let art = ui.interact(rect, egui::Id::new("sidebar-art"), Sense::click());
    let chevron_rect = Rect::from_center_size(
        pos2(rect.right() - 16.0, rect.top() + 16.0),
        Vec2::splat(20.0),
    );
    let over_chevron = ui.rect_contains_pointer(chevron_rect);
    if art.hovered() || over_chevron {
        let chevron = ui.interact(
            chevron_rect,
            egui::Id::new("sidebar-art-collapse"),
            Sense::click(),
        );
        ui.painter().circle_filled(
            chevron_rect.center(),
            10.0,
            palette.panel.gamma_multiply(0.9),
        );
        Icon::ChevronDown.image(palette.text, 14.0).paint_at(
            ui,
            Rect::from_center_size(chevron_rect.center(), Vec2::splat(14.0)),
        );
        if chevron.clicked() {
            app.settings.art_expanded = false;
            app.actions.push(Action::SettingsChanged);
        }
    }
    if art.clicked() && !over_chevron {
        if let Some(id) = album_id {
            app.actions.push(Action::Open(Page::Album(id)));
        } else if let Some(id) = show_id {
            app.actions.push(Action::Open(Page::Show(id)));
        }
    }
}

/// How many playlists a folder holds, nested ones included.
fn folder_playlists(rootlist: &[crate::player::RootlistEntry], id: &str) -> usize {
    use crate::player::RootlistEntry;
    let mut counting = false;
    let mut depth = 0usize;
    let mut count = 0;
    for row in rootlist {
        match row {
            RootlistEntry::FolderStart { id: this, .. } => {
                if counting {
                    depth += 1;
                } else if this == id {
                    counting = true;
                    depth = 1;
                }
            }
            RootlistEntry::FolderEnd if counting => {
                depth -= 1;
                if depth == 0 {
                    return count;
                }
            }
            RootlistEntry::Playlist(_) if counting => count += 1,
            _ => {}
        }
    }
    count
}

/// The artwork widths a row and a grid card ask for on this display.
#[derive(Clone, Copy)]
struct EntryArt {
    row: u32,
    grid: u32,
}

impl EntryArt {
    fn of(app: &App) -> Self {
        Self {
            row: app.art_px(ROW_ART),
            grid: app.art_px(super::widgets::CARD_ART),
        }
    }
}

fn playlist_entry(
    locale: Locale,
    playlist: &crate::api::models::Playlist,
    index: usize,
    user_id: &str,
    editable: bool,
    depth: u8,
    art: EntryArt,
) -> Entry {
    Entry {
        image: pick_image(&playlist.images, art.row).map(str::to_string),
        grid_image: pick_image(&playlist.images, art.grid).map(str::to_string),
        name: playlist.name.clone(),
        // Translators: {owner} is the name of the playlist's owner.
        subtitle: gettext(locale, "Playlist • {owner}").replace("{owner}", playlist.owner_name()),
        grid_subtitle: playlist.owner_name().to_string(),
        page: Page::Playlist(playlist.id.clone()),
        uri: playlist.uri.clone(),
        round: false,
        liked: false,
        owned: playlist.owned_by(user_id),
        editable,
        playlist_index: Some(index),
        folder: None,
        depth,
    }
}

fn nav_row(
    ui: &mut egui::Ui,
    palette: &Palette,
    icon: Icon,
    label: &str,
    active: bool,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(vec2(ui.available_width(), 40.0), Sense::click());
    if ui.is_rect_visible(rect) {
        let color = if active || response.hovered() {
            palette.text
        } else {
            palette.secondary
        };
        let icon_rect =
            Rect::from_center_size(pos2(rect.left() + 22.0, rect.center().y), Vec2::splat(22.0));
        icon.image(color, 22.0).paint_at(ui, icon_rect);
        ui.painter().text(
            pos2(rect.left() + 46.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            label,
            theme::bold(15.0),
            color,
        );
    }
    response.widget_info(|| {
        egui::WidgetInfo::selected(egui::WidgetType::Button, ui.is_enabled(), active, label)
    });
    theme::focus_ring(ui, &response);
    response
}

/// Draw Home, Search and the local pins. The rest of the library stays off the sidebar.
/// Resolve each saved pin independently. Missing or rate-limited library data
/// cannot erase a shortcut or let an unpinned followed item enter the sidebar.
fn pinned_entries(app: &App) -> Vec<Entry> {
    let mut seen = std::collections::HashSet::new();
    let mut entries = Vec::new();
    for key in app.settings.library_pins() {
        if app.audiobook_shows.contains(&key) || !seen.insert(key.clone()) {
            continue;
        }
        let Some(entry) = pin_entry(app, &key) else {
            continue;
        };
        let folder = entry.folder.clone();
        entries.push(entry);
        if let Some((id, false, _)) = folder {
            let mut inside = false;
            let mut depth = 0usize;
            for row in &app.rootlist {
                use crate::player::RootlistEntry;
                match row {
                    RootlistEntry::FolderStart { id: held, .. } if !inside => {
                        if *held == id {
                            inside = true;
                            depth = 1;
                        }
                    }
                    RootlistEntry::FolderStart { .. } if inside => depth += 1,
                    RootlistEntry::FolderEnd if inside => {
                        depth -= 1;
                        if depth == 0 {
                            break;
                        }
                    }
                    RootlistEntry::Playlist(uri) if inside => {
                        if let Some(mut child) = pin_entry(app, uri) {
                            child.depth = 1;
                            entries.push(child);
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    entries
}

fn pin_entry(app: &App, key: &str) -> Option<Entry> {
    if key == LIKED_SONGS_KEY {
        return Some(liked_entry(app));
    }
    if key == "spotifast:episodes" {
        let mut entry = liked_entry(app);
        entry.name = gettext(app.locale, "Your Episodes").into_owned();
        entry.subtitle = gettext(app.locale, "Saved episodes").into_owned();
        entry.page = Page::Episodes;
        entry.uri = key.to_string();
        entry.liked = false;
        return Some(entry);
    }
    let details = app
        .settings
        .library_pin_details
        .get(key)
        .cloned()
        .unwrap_or_default();
    let mut parts = key.splitn(3, ':');
    if parts.next()? != "spotify" {
        return None;
    }
    let kind = parts.next()?;
    let id = parts.next()?;
    let page = match kind {
        "playlist" => Page::Playlist(id.to_string()),
        "album" => Page::Album(id.to_string()),
        "artist" => Page::Artist(id.to_string()),
        "show" => Page::Show(id.to_string()),
        "folder" => Page::Home,
        _ => return None,
    };
    let mut entry = Entry {
        image: details.image.clone(),
        grid_image: details.image,
        name: if details.name.is_empty() {
            id.to_string()
        } else {
            details.name
        },
        subtitle: details.subtitle,
        grid_subtitle: String::new(),
        page,
        uri: key.to_string(),
        round: kind == "artist",
        liked: false,
        owned: false,
        editable: false,
        playlist_index: None,
        folder: None,
        depth: 0,
    };
    let fallback_name = entry.name.clone();
    let fallback_image = entry.image.clone();
    let fallback_subtitle = entry.subtitle.clone();
    let art = EntryArt::of(app);
    match kind {
        "playlist" => {
            let index = app
                .library
                .playlists
                .get()
                .and_then(|items| items.iter().position(|item| item.uri == key));
            let playlist = index
                .and_then(|index| app.library.playlists.get().map(|items| &items[index]))
                .or_else(|| {
                    app.playlist_pages
                        .get(id)
                        .and_then(|page| page.playlist.get())
                });
            if let Some(playlist) = playlist {
                entry = playlist_entry(
                    app.locale,
                    playlist,
                    index.unwrap_or(0),
                    app.user_id().unwrap_or(""),
                    app.can_edit_playlist(playlist),
                    0,
                    art,
                );
                entry.playlist_index = index;
            }
        }
        "album" => {
            let album = app
                .library
                .albums
                .items
                .iter()
                .find(|saved| saved.album.uri == key)
                .map(|saved| &saved.album)
                .or_else(|| app.album_pages.get(id).and_then(|page| page.album.get()));
            if let Some(album) = album {
                entry.name = album.name.clone();
                entry.subtitle = crate::api::models::join_names(
                    album.artists.iter().map(|artist| artist.name.as_str()),
                );
                entry.image = pick_image(&album.images, art.row).map(str::to_string);
                entry.grid_image = pick_image(&album.images, art.grid).map(str::to_string);
            }
        }
        "artist" => {
            let artist = app
                .library
                .artists
                .items
                .iter()
                .find(|artist| artist.uri == key)
                .or_else(|| app.artist_pages.get(id).and_then(|page| page.artist.get()));
            if let Some(artist) = artist {
                entry.name = artist.name.clone();
                entry.image = pick_image(&artist.images, art.row).map(str::to_string);
                entry.grid_image = pick_image(&artist.images, art.grid).map(str::to_string);
            }
        }
        "show" => {
            let show = app
                .library
                .shows
                .items
                .iter()
                .find(|saved| saved.show.uri == key)
                .map(|saved| &saved.show)
                .or_else(|| app.show_pages.get(id).and_then(|page| page.show.get()));
            if let Some(show) = show {
                entry.name = show.name.clone();
                entry.subtitle = show.publisher.clone();
                entry.image = pick_image(&show.images, art.row).map(str::to_string);
                entry.grid_image = pick_image(&show.images, art.grid).map(str::to_string);
            }
        }
        "folder" => {
            let name = app.rootlist.iter().find_map(|row| match row {
                crate::player::RootlistEntry::FolderStart { id: held, name } if held == id => {
                    Some(name)
                }
                _ => None,
            });
            if let Some(name) = name {
                entry.name = name.clone();
            }
            let count = folder_playlists(&app.rootlist, id);
            entry.subtitle = app.locale.folder_playlist_count(count as u32);
            entry.grid_subtitle = app.locale.playlist_count(count as u32);
            entry.folder = Some((
                id.to_string(),
                app.collapsed_folders.contains(&id.to_string()),
                count,
            ));
        }
        _ => {}
    }
    if entry.name.trim().is_empty() {
        entry.name = fallback_name;
    }
    if entry.image.is_none() {
        entry.image = fallback_image.clone();
    }
    if entry.grid_image.is_none() {
        entry.grid_image = fallback_image;
    }
    if entry.subtitle.is_empty() {
        entry.subtitle = fallback_subtitle;
    }
    Some(entry)
}

fn contents(app: &mut App, ui: &mut egui::Ui, grid_art: Option<Rect>) {
    let palette = app.palette;
    let page = app.page().clone();
    let locale = app.locale;
    ui.add_space(4.0);
    if nav_row(
        ui,
        &palette,
        Icon::House,
        &gettext(locale, "Home"),
        page == Page::Home,
    )
    .clicked()
    {
        app.actions.push(Action::Open(Page::Home));
    }
    if nav_row(
        ui,
        &palette,
        Icon::Search,
        &gettext(locale, "Search"),
        page == Page::Search,
    )
    .clicked()
    {
        app.actions.push(Action::FocusSearch);
    }
    ui.add_space(10.0);
    ui.painter().hline(
        ui.max_rect().x_range().shrink(4.0),
        ui.cursor().top(),
        egui::Stroke::new(1.0, palette.outline),
    );
    ui.add_space(10.0);

    // Pins are the complete sidebar. Hidden legacy filters and sort choices
    // must not exclude pins, and no library request is needed to draw them.
    let entries = pinned_entries(app);
    let pins = app.settings.library_pins();
    let custom_order = false;

    let playing_context = app.playing_context_uri();
    let context_playing = app.believed_playing();
    let current_page = app.page().clone();

    crate::autoscroll::show(
        ui,
        egui::ScrollArea::vertical()
            .id_salt("sidebar-list")
            .auto_shrink([false, false]),
        egui::Vec2b::new(false, true),
        |ui| {
            if egui::DragAndDrop::has_payload_of_type::<DragTrack>(ui.ctx())
                || egui::DragAndDrop::has_payload_of_type::<DragEntry>(ui.ctx())
            {
                super::widgets::scroll_during_drag(ui);
            }
            if app.settings.sidebar_grid {
                library_grid(app, ui, &entries, custom_order, grid_art);
                ui.add_space(grid_art.map_or(0.0, |rect| ui.max_rect().bottom() - rect.top()));
                return;
            }

            let compact = app.settings.sidebar_compact;
            let row_height = if compact {
                COMPACT_ROW_HEIGHT
            } else {
                DEFAULT_ROW_HEIGHT
            };
            // Calculate drop positions from fixed row height because rows shift
            // before drawing.
            let list_top = ui.cursor().top();
            let pointer = ui.ctx().pointer_latest_pos().filter(|pos| {
                ui.clip_rect().contains(*pos) && ui.rect_contains_pointer(ui.clip_rect())
            });
            // Tracks may drop on Liked Songs or playlists that take songs
            // from this account.
            let dragging_song = egui::DragAndDrop::has_payload_of_type::<DragTrack>(ui.ctx());
            let drop_target = dragging_song
                .then_some(pointer)
                .flatten()
                .map(|pos| ((pos.y - list_top) / row_height).floor())
                .filter(|row| *row >= 0.0 && *row < entries.len() as f32)
                .map(|row| row as usize)
                .filter(|row| entries[*row].liked || entries[*row].editable);
            // Sidebar entries, including Liked Songs, drop between rows.
            let reordering = egui::DragAndDrop::has_payload_of_type::<DragEntry>(ui.ctx());
            let reorder_slot = reordering.then_some(pointer).flatten().map(|pos| {
                (((pos.y - list_top) / row_height).round().max(0.0) as usize).min(entries.len())
            });
            let art = app.backend.art().clone();
            super::widgets::virtual_rows(ui, entries.len(), row_height, |ui, index| {
                let entry = &entries[index];
                if !app.settings.sidebar_compact
                    && let Some(image) = &entry.image
                {
                    // Prepare the enlarged preview before this row is opened.
                    app.softened_covers.texture(ui.ctx(), &art, image);
                }
                let droppable = entry.liked || entry.editable;
                let drop_hover = drop_target == Some(index);
                let active = entry.folder.is_none() && entry.page == current_page;
                // Liked Songs has no URI of its own here; Spotify plays it
                // as the account's collection context.
                let current = entry_is_playing_context(entry, playing_context.as_deref());
                let playing = context_playing && current;
                let pinned = pins.iter().any(|key| key == entry.ordering_key());
                let (_, rect) = ui.allocate_space(vec2(ui.available_width(), row_height));
                let id = ui.id().with((
                    "library-row",
                    &entry.uri,
                    entry.liked,
                    entry.folder.as_ref().map(|(id, _, _)| id),
                ));
                let response = ui.interact(rect, id, Sense::click_and_drag());
                response.widget_info(|| {
                    egui::WidgetInfo::selected(
                        egui::WidgetType::Button,
                        ui.is_enabled(),
                        active,
                        if let Some((_, collapsed, _)) = &entry.folder {
                            app.locale.folder_state_label(&entry.name, *collapsed)
                        } else {
                            entry.name.clone()
                        },
                    )
                });
                // Animate rows around the current track or entry drop target.
                let shift = ui.ctx().animate_value_with_time(
                    ui.id().with(("drop-shift", index)),
                    if let Some(slot) = reorder_slot {
                        if index < slot { -4.0 } else { 4.0 }
                    } else {
                        match drop_target {
                            Some(target) if index < target => -4.0,
                            Some(target) if index > target => 4.0,
                            _ => 0.0,
                        }
                    },
                    crate::motion::seconds(ui.ctx(), 0.12),
                );
                let rect = rect.translate(vec2(0.0, shift));
                // Set when the cover play button takes a click, so a double
                // click on it does not also play from the row.
                let mut cover_took_click = false;
                if ui.is_rect_visible(rect) {
                    if active {
                        ui.painter()
                            .rect_filled(rect, CornerRadius::same(6), palette.surface);
                    } else if response.hovered() {
                        ui.painter().rect_filled(
                            rect,
                            CornerRadius::same(6),
                            palette.surface_hover.gamma_multiply(0.6),
                        );
                    }
                    if drop_hover {
                        ui.painter().rect_filled(
                            rect,
                            CornerRadius::same(6),
                            palette.accent.gamma_multiply(0.18),
                        );
                        ui.painter().rect_stroke(
                            rect,
                            CornerRadius::same(6),
                            egui::Stroke::new(1.5, palette.accent),
                            egui::StrokeKind::Inside,
                        );
                    }
                    let name_color = if playing {
                        palette.accent
                    } else {
                        palette.text
                    };
                    let indent = f32::from(entry.depth) * 14.0;
                    if let Some((_, collapsed, _)) = &entry.folder {
                        let chevron = if *collapsed {
                            Icon::ChevronRight
                        } else {
                            Icon::ChevronDown
                        };
                        let left = rect.left() + LIBRARY_ITEM_PADDING + indent;
                        chevron.image(palette.secondary, 16.0).paint_at(
                            ui,
                            Rect::from_center_size(
                                pos2(left + 8.0, rect.center().y),
                                Vec2::splat(16.0),
                            ),
                        );
                        Icon::Library.image(palette.secondary, 20.0).paint_at(
                            ui,
                            Rect::from_center_size(
                                pos2(left + 30.0, rect.center().y),
                                Vec2::splat(20.0),
                            ),
                        );
                        let text_left = left + 46.0;
                        let text_right = rect.right() - LIBRARY_ITEM_PADDING;
                        let painter = ui.painter().with_clip_rect(Rect::from_min_max(
                            pos2(text_left, rect.top()),
                            pos2(text_right, rect.bottom()),
                        ));
                        crate::bidi::paint_line(
                            &painter,
                            text_left,
                            text_right,
                            rect.center().y - if compact { 0.0 } else { 9.0 },
                            &entry.name,
                            theme::medium(if compact { 13.5 } else { 14.0 }),
                            name_color,
                        );
                        if !compact {
                            crate::bidi::paint_line(
                                &painter,
                                text_left,
                                text_right,
                                rect.center().y + 10.0,
                                &entry.subtitle,
                                theme::regular(12.5),
                                palette.secondary,
                            );
                        }
                    } else if compact {
                        let text_left = rect.left() + LIBRARY_ITEM_PADDING + indent;
                        let text_right = rect.right()
                            - if playing || pinned {
                                28.0
                            } else {
                                LIBRARY_ITEM_PADDING
                            };
                        let painter = ui.painter().with_clip_rect(Rect::from_min_max(
                            pos2(text_left, rect.top()),
                            pos2(text_right, rect.bottom()),
                        ));
                        crate::bidi::paint_line(
                            &painter,
                            text_left,
                            text_right,
                            rect.center().y,
                            &entry.name,
                            theme::medium(13.5),
                            name_color,
                        );
                    } else {
                        let cover_rect = Rect::from_center_size(
                            pos2(
                                rect.left() + LIBRARY_ITEM_PADDING + indent + 22.0,
                                rect.center().y,
                            ),
                            Vec2::splat(ROW_ART),
                        );
                        if entry.liked {
                            liked_cover(ui, cover_rect, 6.0);
                        } else if entry.page == Page::Episodes {
                            episodes_cover(ui, cover_rect, 6.0);
                        } else {
                            super::widgets::paint_cover(
                                ui,
                                &palette,
                                entry.image.as_deref(),
                                cover_rect,
                                if entry.round { 22.0 } else { 6.0 },
                                if entry.round { Icon::User } else { Icon::Music },
                                Some(app.backend.art()),
                            );
                        }
                        let text_left = cover_rect.right() + 12.0;
                        let text_right = rect.right()
                            - if playing || pinned {
                                28.0
                            } else {
                                LIBRARY_ITEM_PADDING
                            };
                        let painter = ui.painter().with_clip_rect(Rect::from_min_max(
                            pos2(text_left, rect.top()),
                            pos2(text_right, rect.bottom()),
                        ));
                        crate::bidi::paint_line(
                            &painter,
                            text_left,
                            text_right,
                            rect.center().y - 9.0,
                            &entry.name,
                            theme::medium(14.0),
                            name_color,
                        );
                        crate::bidi::paint_line(
                            &painter,
                            text_left,
                            text_right,
                            rect.center().y + 10.0,
                            &entry.subtitle,
                            theme::regular(12.5),
                            palette.secondary,
                        );
                        // Hovering the art offers to play right from here.
                        cover_took_click = cover_play_button(
                            app, ui, entry, index, cover_rect, &response, current,
                        );
                    }
                    if playing {
                        let icon_rect = Rect::from_center_size(
                            pos2(rect.right() - 16.0, rect.center().y),
                            Vec2::splat(16.0),
                        );
                        Icon::Volume2
                            .image(palette.accent, 16.0)
                            .paint_at(ui, icon_rect);
                    } else if pinned {
                        let icon_rect = Rect::from_center_size(
                            pos2(rect.right() - 16.0, rect.center().y),
                            Vec2::splat(13.0),
                        );
                        Icon::Pin
                            .image(palette.secondary, 13.0)
                            .paint_at(ui, icon_rect);
                    }
                    // Rows that cannot take the song step back a little.
                    if dragging_song && !droppable {
                        ui.painter().rect_filled(
                            rect,
                            CornerRadius::same(6),
                            palette.panel.gamma_multiply(0.5),
                        );
                    }
                }
                finish_entry_interaction(
                    app,
                    ui,
                    &response,
                    entry,
                    cover_took_click,
                    custom_order,
                    true,
                );
            });
            if let Some(slot) = reorder_slot {
                // A line in the gap the rows opened, so the eye lands
                // where the row will.
                let y = list_top + slot as f32 * row_height;
                ui.painter().hline(
                    ui.max_rect().x_range().shrink(6.0),
                    y,
                    egui::Stroke::new(2.0, palette.accent),
                );
                if ui.input(|input| input.pointer.button_released(egui::PointerButton::Primary))
                    && let Some(drag) = egui::DragAndDrop::take_payload::<DragEntry>(ui.ctx())
                {
                    drop_pin_row(app, &entries, slot, &drag.uri);
                }
            }
        },
    );
}

const EXPANDED_ART_GAP: f32 = 10.0;
const LIBRARY_ITEM_PADDING: f32 = 8.0;
/// The cover of a list row.
const ROW_ART: f32 = 44.0;
const GRID_MIN_CARD_WIDTH: f32 = 108.0;
const GRID_MAX_COLUMNS: usize = 4;
const GRID_TEXT_HEIGHT: f32 = 44.0;

#[derive(Clone, Copy)]
struct GridLayout {
    columns: usize,
    card_width: f32,
    card_height: f32,
    row_height: f32,
}

fn grid_layout(width: f32) -> GridLayout {
    let columns = ((width / GRID_MIN_CARD_WIDTH).floor() as usize).clamp(2, GRID_MAX_COLUMNS);
    let card_width = (width / columns as f32).max(1.0);
    let card_height = card_width + GRID_TEXT_HEIGHT;
    GridLayout {
        columns,
        card_width,
        card_height,
        row_height: card_height,
    }
}

fn paint_grid_text(
    ui: &egui::Ui,
    rect: Rect,
    text: &str,
    font: egui::FontId,
    color: egui::Color32,
) {
    let galley = crate::bidi::layout(
        ui.painter(),
        text,
        font,
        color,
        rect.width(),
        1,
        Some(crate::bidi::ELLIPSIS),
    );
    ui.painter()
        .galley(crate::bidi::galley_pos(rect, &galley), galley, color);
}

fn grid_reorder_slot(
    pointer: egui::Pos2,
    origin: egui::Pos2,
    layout: GridLayout,
    count: usize,
) -> usize {
    let row = ((pointer.y - origin.y) / layout.row_height)
        .floor()
        .max(0.0) as usize;
    let column = ((pointer.x - origin.x) / layout.card_width)
        .floor()
        .max(0.0)
        .min((layout.columns - 1) as f32) as usize;
    let after = pointer.x - origin.x - column as f32 * layout.card_width > layout.card_width / 2.0;
    (row * layout.columns + column + usize::from(after)).min(count)
}

fn library_grid(
    app: &mut App,
    ui: &mut egui::Ui,
    entries: &[Entry],
    custom_order: bool,
    grid_art: Option<Rect>,
) {
    if entries.is_empty() {
        return;
    }
    let palette = app.palette;
    let pins = app.settings.library_pins();
    let playing_context = app.playing_context_uri();
    let context_playing = app.believed_playing();
    let current_page = app.page().clone();
    let layout = grid_layout(ui.available_width());
    let origin = ui.cursor().min;
    let pointer = ui.ctx().pointer_latest_pos().filter(|pos| {
        ui.clip_rect().contains(*pos)
            && ui.rect_contains_pointer(ui.clip_rect())
            && !grid_art.is_some_and(|art| art.contains(*pos))
    });
    let dragging_song = egui::DragAndDrop::has_payload_of_type::<DragTrack>(ui.ctx());
    let reordering = egui::DragAndDrop::has_payload_of_type::<DragEntry>(ui.ctx());
    let reorder_slot = reordering
        .then_some(pointer)
        .flatten()
        .map(|pointer| grid_reorder_slot(pointer, origin, layout, entries.len()));
    let row_count = entries.len().div_ceil(layout.columns);
    let grid_id = ui.unique_id().with("library-grid");

    super::widgets::virtual_rows(ui, row_count, layout.row_height, |ui, row| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let start = row * layout.columns;
            let end = (start + layout.columns).min(entries.len());
            for index in start..end {
                let entry = &entries[index];
                let entry_id = (
                    entry.ordering_key(),
                    entry.liked,
                    entry.folder.as_ref().map(|(id, _, _)| id.as_str()),
                );
                ui.scope_builder(egui::UiBuilder::new().id(grid_id.with(entry_id)), |ui| {
                    let active = entry.folder.is_none() && entry.page == current_page;
                    let playing_here = entry_is_playing_context(entry, playing_context.as_deref());
                    let playing = context_playing && playing_here;
                    let pinned = pins.iter().any(|key| key == entry.ordering_key());
                    let droppable = entry.liked || entry.editable;
                    let (rect, response) = ui.allocate_exact_size(
                        vec2(layout.card_width, layout.card_height),
                        Sense::click_and_drag(),
                    );
                    response.widget_info(|| {
                        egui::WidgetInfo::selected(
                            egui::WidgetType::Button,
                            ui.is_enabled(),
                            active,
                            if let Some((_, collapsed, _)) = &entry.folder {
                                app.locale.folder_state_label(&entry.name, *collapsed)
                            } else {
                                entry.name.clone()
                            },
                        )
                    });
                    let cover_rect = Rect::from_min_size(
                        rect.min + Vec2::splat(LIBRARY_ITEM_PADDING),
                        Vec2::splat(layout.card_width - LIBRARY_ITEM_PADDING * 2.0),
                    );
                    let mut cover_took_click = false;
                    if ui.is_rect_visible(rect) {
                        if active {
                            ui.painter()
                                .rect_filled(rect, CornerRadius::same(6), palette.surface);
                        } else if response.hovered() {
                            ui.painter().rect_filled(
                                rect,
                                CornerRadius::same(6),
                                palette.surface_hover.gamma_multiply(0.6),
                            );
                        }
                        if entry.liked {
                            liked_cover(ui, cover_rect, 6.0);
                        } else if entry.page == Page::Episodes {
                            episodes_cover(ui, cover_rect, 6.0);
                        } else if entry.folder.is_some() {
                            ui.painter().rect_filled(
                                cover_rect,
                                CornerRadius::same(6),
                                palette.surface,
                            );
                            Icon::Library
                                .image(palette.secondary, layout.card_width * 0.3)
                                .paint_at(
                                    ui,
                                    Rect::from_center_size(
                                        cover_rect.center(),
                                        Vec2::splat(layout.card_width * 0.3),
                                    ),
                                );
                        } else {
                            super::widgets::paint_cover(
                                ui,
                                &palette,
                                entry.grid_image.as_deref().or(entry.image.as_deref()),
                                cover_rect,
                                if entry.round {
                                    layout.card_width / 2.0
                                } else {
                                    6.0
                                },
                                if entry.round { Icon::User } else { Icon::Music },
                                Some(app.backend.art()),
                            );
                        }

                        cover_took_click =
                            grid_play_button(app, ui, entry, cover_rect, &response, playing_here);

                        let text_left = rect.left() + LIBRARY_ITEM_PADDING;
                        let text_width = (rect.width() - LIBRARY_ITEM_PADDING * 2.0).max(0.0);
                        paint_grid_text(
                            ui,
                            Rect::from_min_size(
                                pos2(text_left, cover_rect.bottom() + 4.0),
                                vec2(text_width, 18.0),
                            ),
                            &entry.name,
                            theme::medium(13.5),
                            if playing {
                                palette.accent
                            } else {
                                palette.text
                            },
                        );
                        paint_grid_text(
                            ui,
                            Rect::from_min_size(
                                pos2(text_left, cover_rect.bottom() + 23.0),
                                vec2(text_width, 17.0),
                            ),
                            &entry.grid_subtitle,
                            theme::regular(12.0),
                            palette.secondary,
                        );

                        if pinned {
                            let pin_rect = grid_pin_rect(cover_rect);
                            ui.painter().circle_filled(
                                pin_rect.center(),
                                pin_rect.width() / 2.0,
                                egui::Color32::from_black_alpha(170),
                            );
                            Icon::Pin.image(egui::Color32::WHITE, 12.0).paint_at(
                                ui,
                                Rect::from_center_size(pin_rect.center(), Vec2::splat(12.0)),
                            );
                        }
                        if dragging_song && !droppable {
                            ui.painter().rect_filled(
                                rect,
                                CornerRadius::same(6),
                                palette.panel.gamma_multiply(0.5),
                            );
                        } else if dragging_song
                            && droppable
                            && pointer.is_some_and(|pos| rect.contains(pos))
                        {
                            ui.painter().rect_stroke(
                                cover_rect,
                                CornerRadius::same(6),
                                egui::Stroke::new(2.0, palette.accent),
                                egui::StrokeKind::Inside,
                            );
                        }
                        if reorder_slot == Some(index) {
                            ui.painter().vline(
                                rect.left(),
                                rect.y_range(),
                                egui::Stroke::new(2.0, palette.accent),
                            );
                        }
                        if index + 1 == entries.len() && reorder_slot == Some(entries.len()) {
                            ui.painter().vline(
                                rect.right(),
                                rect.y_range(),
                                egui::Stroke::new(2.0, palette.accent),
                            );
                        }
                    }

                    finish_entry_interaction(
                        app,
                        ui,
                        &response,
                        entry,
                        cover_took_click,
                        custom_order,
                        pointer.is_some(),
                    );
                });
            }
        });
    });

    if let Some(slot) = reorder_slot
        && ui.input(|input| input.pointer.button_released(egui::PointerButton::Primary))
        && let Some(drag) = egui::DragAndDrop::take_payload::<DragEntry>(ui.ctx())
    {
        drop_pin_row(app, entries, slot, &drag.uri);
    }
}

fn entry_menu(app: &mut App, response: &egui::Response, entry: &Entry, custom_order: bool) {
    if entry.folder.is_some() || entry.page == Page::Episodes {
        egui::Popup::context_menu(response)
            .frame(super::widgets::menu_frame(&app.palette))
            .show(|ui| {
                pin_menu(app, ui, entry.ordering_key(), &entry.name);
            });
    } else if !entry.uri.is_empty() {
        let owned_playlist = entry
            .owned
            .then_some(entry.playlist_index)
            .flatten()
            .and_then(|index| {
                app.library
                    .playlists
                    .get()
                    .and_then(|list| list.get(index))
                    .cloned()
            });
        egui::Popup::context_menu(response)
            .frame(super::widgets::menu_frame(&app.palette))
            .show(|ui| {
                super::widgets::context_menu_items(
                    ui,
                    app,
                    &entry.uri,
                    &entry.name,
                    owned_playlist.as_ref(),
                );
                if custom_order
                    && super::widgets::menu_item(
                        ui,
                        &app.palette,
                        Some(Icon::Clock),
                        &gettext(app.locale, "Sort by recently played"),
                    )
                {
                    app.actions.push(Action::SetLibrarySort {
                        shelf: Filter::Playlists,
                        sort: LibrarySort::RecentlyPlayed,
                    });
                }
            });
    } else if entry.liked {
        egui::Popup::context_menu(response)
            .frame(super::widgets::menu_frame(&app.palette))
            .show(|ui| {
                // The same width as every other menu; without it the menu
                // stretches as wide as the window.
                ui.set_min_width(200.0);
                ui.set_max_width(300.0);
                if super::widgets::menu_item(
                    ui,
                    &app.palette,
                    Some(Icon::Play),
                    &gettext(app.locale, "Play"),
                ) && let Some(user) = &app.user
                {
                    app.actions.push(Action::PlayContext {
                        uri: format!("spotify:user:{}:collection", user.id),
                        offset_uri: None,
                        offset_index: None,
                    });
                }
                pin_menu(app, ui, entry.ordering_key(), &entry.name);
            });
    }
}

pub(crate) fn pin_menu(app: &mut App, ui: &mut egui::Ui, key: &str, name: &str) {
    if key.is_empty() {
        return;
    }
    let pinned = app.settings.library_pins().iter().any(|held| held == key);
    let label = if pinned {
        gettext(app.locale, "Unpin")
    } else {
        gettext(app.locale, "Pin to top")
    };
    if super::widgets::menu_item(
        ui,
        &app.palette,
        Some(if pinned { Icon::PinOff } else { Icon::Pin }),
        &label,
    ) {
        let entry = pin_entry(app, key);
        let details = crate::settings::LibraryPinDetails {
            name: name.to_string(),
            image: entry.as_ref().and_then(|entry| entry.image.clone()),
            subtitle: entry.map_or_else(String::new, |entry| entry.subtitle),
        };
        app.actions.push(Action::ToggleLibraryPin {
            uri: key.to_string(),
            details,
        });
    }
}

/// Translate visible gaps to pin order, ignoring a pinned folder's children.
fn drop_pin_row(app: &mut App, entries: &[Entry], slot: usize, key: &str) {
    let roots: Vec<Entry> = pinned_entries(app)
        .into_iter()
        .filter(|entry| entry.depth == 0)
        .collect();
    let slot = entries
        .iter()
        .take(slot)
        .filter(|entry| entry.depth == 0)
        .count();
    drop_row(app, &roots, roots.len(), slot, key);
}

/// Reorders the pin block without disturbing pins on another shelf.
fn drop_row(app: &mut App, entries: &[Entry], pinned_rows: usize, slot: usize, key: &str) {
    let mut pins = app.settings.library_pins();
    pins.retain(|held| held != key);
    if slot <= pinned_rows {
        let anchor = entries[..pinned_rows]
            .iter()
            .skip(slot)
            .map(Entry::ordering_key)
            .find(|held| *held != key);
        let at = anchor
            .and_then(|anchor| pins.iter().position(|held| held == anchor))
            .unwrap_or(pins.len());
        pins.insert(at, key.to_string());
    }
    app.actions.push(Action::ArrangeLibrary {
        pinned: pins,
        playlist_order: None,
    });
}

/// The green bookmark tile for the account's saved episodes.
fn episodes_cover(ui: &egui::Ui, rect: Rect, radius: f32) {
    ui.painter().rect_filled(
        rect,
        CornerRadius::same(radius.min(127.0) as u8),
        egui::Color32::from_rgb(0, 100, 80),
    );
    let size = rect.width() * 0.5;
    Icon::BookmarkFilled
        .image(egui::Color32::from_rgb(30, 215, 96), size)
        .paint_at(ui, Rect::from_center_size(rect.center(), Vec2::splat(size)));
}

/// The purple-to-blue Liked Songs tile.
pub fn liked_cover(ui: &egui::Ui, rect: Rect, radius: f32) {
    let texture_id = egui::Id::new("liked-cover-gradient");
    let texture = ui
        .data(|data| data.get_temp::<egui::TextureHandle>(texture_id))
        .unwrap_or_else(|| {
            let size = 64;
            let lerp = |a: u8, b: u8, t: f32| (a as f32 + (b as f32 - a as f32) * t) as u8;
            let top_left = [0x45, 0x0a, 0xf5];
            let top_right = [0x6a, 0x3a, 0xe8];
            let bottom_left = [0x8e, 0x9f, 0xe5];
            let bottom_right = [0xc4, 0xef, 0xd9];
            let pixels = (0..size)
                .flat_map(|y| {
                    let y = y as f32 / (size - 1) as f32;
                    (0..size).map(move |x| {
                        let x = x as f32 / (size - 1) as f32;
                        egui::Color32::from_rgb(
                            lerp(
                                lerp(top_left[0], top_right[0], x),
                                lerp(bottom_left[0], bottom_right[0], x),
                                y,
                            ),
                            lerp(
                                lerp(top_left[1], top_right[1], x),
                                lerp(bottom_left[1], bottom_right[1], x),
                                y,
                            ),
                            lerp(
                                lerp(top_left[2], top_right[2], x),
                                lerp(bottom_left[2], bottom_right[2], x),
                                y,
                            ),
                        )
                    })
                })
                .collect();
            let texture = ui.ctx().load_texture(
                "liked-cover-gradient",
                egui::ColorImage::new([size, size], pixels),
                egui::TextureOptions::LINEAR,
            );
            ui.data_mut(|data| data.insert_temp(texture_id, texture.clone()));
            texture
        });
    egui::Image::new(&texture)
        .corner_radius(CornerRadius::same(radius.min(127.0) as u8))
        .paint_at(ui, rect);
    let size = rect.width() * 0.45;
    let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(size));
    Icon::HeartFilled
        .image(egui::Color32::WHITE, size)
        .paint_at(ui, icon_rect);
}

#[cfg(all(test, feature = "demo"))]
mod pin_tests {
    use super::*;
    use crate::api::models::Playlist;
    use crate::model::Loadable;
    const TEST_ART: EntryArt = EntryArt { row: 64, grid: 300 };
    use crate::settings::{LibraryPinDetails, Settings};

    fn app(name: &str) -> App {
        let root =
            std::env::temp_dir().join(format!("spotifast-pins-{name}-{}", std::process::id()));
        let mut app = App::new(
            &crate::backend::Waker::default(),
            crate::paths::AppDirs {
                config: root.join("config"),
                state: root.join("state"),
                cache: root.join("cache"),
            },
            Settings::default(),
            crate::app::AppOptions {
                media_controls: false,
                restore_sign_in: false,
                tray: false,
            },
        );
        crate::demo::populate(&mut app);
        app.settings.pinned_contexts.clear();
        app
    }
    fn keys(app: &App) -> Vec<String> {
        pinned_entries(app)
            .iter()
            .map(|entry| entry.ordering_key().to_string())
            .collect()
    }
    fn apply(app: &mut App) {
        for action in std::mem::take(&mut app.actions) {
            app.apply(action, &egui::Context::default());
        }
    }

    #[test]
    fn sidebar_contains_only_pins_in_saved_order_despite_old_filters() {
        let mut app = app("filters");
        app.settings.pinned_contexts = vec![
            "spotify:playlist:pl2".into(),
            LIKED_SONGS_KEY.into(),
            "spotify:playlist:pl4".into(),
        ];
        app.settings.library_by_you = true;
        app.library.filter = "no matching titles".into();
        app.settings
            .library_sort
            .insert(Filter::Playlists, LibrarySort::Name);
        assert_eq!(keys(&app), app.settings.pinned_contexts);
        app.backend.shutdown();
    }
    #[test]
    fn pins_survive_rate_limits_and_settings_round_trip_with_names_and_covers() {
        let mut app = app("rate-limit");
        app.settings.pinned_contexts = vec!["spotify:playlist:saved".into()];
        app.settings.liked_songs_pinned = false;
        app.settings.library_pin_details.insert(
            "spotify:playlist:saved".into(),
            LibraryPinDetails {
                name: "My pinned playlist".into(),
                image: Some("https://example.test/cover.jpg".into()),
                subtitle: "Playlist".into(),
            },
        );
        app.settings =
            serde_json::from_str(&serde_json::to_string(&app.settings).unwrap()).unwrap();
        app.library.playlists = Loadable::Failed("rate limited".into());
        let entries = pinned_entries(&app);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].name, "My pinned playlist");
        assert_eq!(
            entries[0].image.as_deref(),
            Some("https://example.test/cover.jpg")
        );
        assert!(matches!(&entries[0].page,Page::Playlist(id) if id=="saved"));
        app.backend.shutdown();
    }
    #[test]
    fn no_pins_leaves_the_library_area_empty() {
        let mut app = app("empty");
        app.settings.liked_songs_pinned = false;
        app.settings.pinned_contexts.clear();
        assert!(pinned_entries(&app).is_empty());
        app.backend.shutdown();
    }
    #[test]
    fn mixed_pins_are_unique_and_do_not_need_saved_library_membership() {
        let mut app = app("mixed");
        app.settings.liked_songs_pinned = false;
        app.settings.pinned_contexts = [
            "spotify:show:podcast",
            "spotify:album:album",
            "spotify:artist:artist",
            "spotify:playlist:outside",
            "spotify:album:album",
        ]
        .map(str::to_string)
        .to_vec();
        assert_eq!(keys(&app), app.settings.pinned_contexts[..4]);
        assert!(pinned_entries(&app)[2].round);
        app.backend.shutdown();
    }
    #[test]
    fn pinning_and_unpinning_uses_the_real_action_handler_and_keeps_other_pins() {
        let mut app = app("toggle");
        let details = LibraryPinDetails {
            name: "A shortcut".into(),
            ..Default::default()
        };
        app.actions.push(Action::ToggleLibraryPin {
            uri: "spotify:playlist:outside".into(),
            details: details.clone(),
        });
        apply(&mut app);
        assert_eq!(keys(&app), [LIKED_SONGS_KEY, "spotify:playlist:outside"]);
        assert_eq!(pinned_entries(&app)[1].name, "A shortcut");
        app.actions.push(Action::ToggleLibraryPin {
            uri: "spotify:playlist:outside".into(),
            details,
        });
        apply(&mut app);
        assert_eq!(keys(&app), [LIKED_SONGS_KEY]);
        assert!(
            !app.settings
                .library_pin_details
                .contains_key("spotify:playlist:outside")
        );
        app.backend.shutdown();
    }
    #[test]
    fn dragging_to_the_last_gap_reorders_without_unpinning_or_changing_legacy_order() {
        let mut app = app("drag");
        app.settings.pinned_contexts = vec![
            "spotify:playlist:pl1".into(),
            LIKED_SONGS_KEY.into(),
            "spotify:playlist:pl2".into(),
        ];
        app.settings.sidebar_order = vec!["spotify:playlist:pl3".into()];
        let entries = pinned_entries(&app);
        drop_pin_row(&mut app, &entries, entries.len(), "spotify:playlist:pl1");
        apply(&mut app);
        assert_eq!(
            keys(&app),
            [
                LIKED_SONGS_KEY,
                "spotify:playlist:pl2",
                "spotify:playlist:pl1"
            ]
        );
        assert_eq!(app.settings.sidebar_order, ["spotify:playlist:pl3"]);
        app.backend.shutdown();
    }
    #[test]
    fn a_pinned_folder_expands_only_its_contents_and_collapses_without_losing_the_pin() {
        use crate::player::RootlistEntry;
        let mut app = app("folder");
        app.settings.liked_songs_pinned = false;
        app.settings.pinned_contexts =
            vec!["spotify:folder:f1".into(), "spotify:playlist:pl3".into()];
        app.rootlist = vec![
            RootlistEntry::FolderStart {
                id: "f1".into(),
                name: "My folder".into(),
            },
            RootlistEntry::Playlist("spotify:playlist:pl1".into()),
            RootlistEntry::FolderEnd,
            RootlistEntry::Playlist("spotify:playlist:pl2".into()),
        ];
        assert_eq!(
            keys(&app),
            [
                "spotify:folder:f1",
                "spotify:playlist:pl1",
                "spotify:playlist:pl3"
            ]
        );
        assert_eq!(pinned_entries(&app)[0].name, "My folder");
        assert_eq!(entry_play_uri(&app, &pinned_entries(&app)[0]), None);
        app.collapsed_folders.push("f1".into());
        assert_eq!(keys(&app), app.settings.pinned_contexts);
        app.backend.shutdown();
    }
    #[test]
    fn old_settings_keep_liked_songs_and_accept_pin_metadata() {
        let settings: Settings = serde_json::from_str("{}").unwrap();
        assert_eq!(settings.library_pins(), [LIKED_SONGS_KEY]);
        assert!(settings.library_pin_details.is_empty());
    }
    #[test]
    fn grid_cards_use_smaller_art_than_page_headers() {
        let images = [64, 300, 640]
            .map(|width| crate::api::models::Image {
                url: width.to_string(),
                width: Some(width),
                height: Some(width),
            })
            .to_vec();
        let playlist = Playlist {
            images: images.clone(),
            ..Default::default()
        };
        let entry = playlist_entry(Locale::English, &playlist, 0, "", false, 0, TEST_ART);
        assert_eq!(entry.image.as_deref(), Some("64"));
        assert_eq!(entry.grid_image.as_deref(), Some("300"));
        assert_eq!(pick_image(&images, 640), Some("640"));
    }

    #[test]
    fn library_grid_adds_columns_as_the_sidebar_grows() {
        let narrow = grid_layout(230.0);
        let medium = grid_layout(360.0);
        let wide = grid_layout(500.0);
        assert_eq!(narrow.columns, 2);
        assert_eq!(medium.columns, 3);
        assert_eq!(wide.columns, 4);
        for (width, layout) in [(230.0, narrow), (360.0, medium), (500.0, wide)] {
            let filled = layout.card_width * layout.columns as f32;
            assert!((filled - width).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn grid_pin_and_play_controls_keep_opposite_corners() {
        let cover = Rect::from_min_size(pos2(10.0, 20.0), Vec2::splat(GRID_MIN_CARD_WIDTH));
        let pin = grid_pin_rect(cover);
        let play = grid_play_rect(cover);
        assert!(pin.center().x < cover.center().x && pin.center().y < cover.center().y);
        assert!(play.center().x > cover.center().x && play.center().y > cover.center().y);
        assert!(!pin.intersects(play));
    }

    #[test]
    fn grid_drop_positions_follow_visual_row_order() {
        let layout = grid_layout(360.0);
        let origin = pos2(20.0, 40.0);
        assert_eq!(grid_reorder_slot(pos2(21.0, 41.0), origin, layout, 8), 0);
        assert_eq!(
            grid_reorder_slot(
                pos2(origin.x + layout.card_width, origin.y + 1.0),
                origin,
                layout,
                8,
            ),
            1
        );
        assert_eq!(
            grid_reorder_slot(
                pos2(origin.x + 1.0, origin.y + layout.row_height + 1.0),
                origin,
                layout,
                8,
            ),
            3
        );
    }
}
