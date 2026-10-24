// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Application-owned RXUI web host. CAD parsing lives in a separate browser worker.
use crate::project::{Project, Scene};
use rxui::{
    Modifiers, PointerEvent, UiPainter,
    astrelis::{FrameError, GraphicsContext, RenderTarget, SurfaceOptions, wgpu},
    prelude::*,
};
use std::time::Duration;
use wasm_bindgen::prelude::*;
fn error(e: impl std::fmt::Display) -> JsValue {
    JsValue::from_str(&e.to_string())
}
#[derive(Default)]
struct Model {
    paths: Vec<String>,
    search: String,
    selected: usize,
    occurrence_path: Option<String>,
    occurrence_config: Option<String>,
    config: usize,
    configs: Vec<String>,
    status: String,
    info: Vec<String>,
    issues: Vec<String>,
    open: bool,
    demo: bool,
    reset: bool,
    wire: bool,
    hidden: bool,
}
impl View for Model {
    fn view(&self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let mut files = column().fill_width().gap(4.);
        let query = self.search.trim().to_lowercase();
        let mut matches = 0;
        for (i, path) in self.paths.iter().enumerate() {
            if !path.to_lowercase().contains(&query) {
                continue;
            }
            matches += 1;
            let short = path.rsplit('/').next().unwrap_or(path);
            files = files.child(
                button(short)
                    .height(if short.chars().count() > 28 { 56. } else { 36. })
                    .key(format!("file-{i}"))
                    .variant(if i == self.selected {
                        ButtonVariant::Primary
                    } else {
                        ButtonVariant::Default
                    })
                    .on_click(cx.listener(move |s, _, _| {
                        s.occurrence_path = None;
                        s.occurrence_config = None;
                        s.selected = i;
                        s.config = 0;
                        s.configs.clear();
                        s.status = "Loading document…".into();
                    })),
            );
        }
        if matches == 0 && !query.is_empty() {
            files = files.child(label("No matching documents").color(ThemeColor::TextMuted));
        }
        let options: Vec<_> = self
            .configs
            .iter()
            .enumerate()
            .map(|(i, name)| {
                choice(
                    i.to_string(),
                    if name.is_empty() { "Unnamed" } else { name },
                )
            })
            .collect();
        let configs = select(cx)
            .key("configuration")
            .selected(self.config.to_string())
            .options(options)
            .on_change(cx.listener(|s, e: &ChoiceEvent, _| {
                if let Some(index) =
                    (0..s.configs.len()).find(|i| Key::from(i.to_string()) == e.key)
                {
                    s.occurrence_config = None;
                    s.config = index;
                }
            }));
        let mut details = column().fill_width().gap(7.);
        for line in &self.info {
            details = details.child(label(line).font_size(13.));
        }
        for line in self.issues.iter().take(8) {
            details = details.child(label(line).font_size(12.).color([0.95, 0.66, 0.39, 1.]));
        }
        column().fill_width().fill_height().min_height(0.).padding(18.).gap(12.)
            .child(label("SW / EXPLORER").font_size(25.))
            .child(label("Local CAD archive viewer").color(ThemeColor::TextMuted))
            .child(button("Open archive or Part…").key("open").variant(ButtonVariant::Primary).on_click(cx.listener(|s,_,_|s.open=true)))
            .child(button("Load synthetic demo").key("demo").on_click(cx.listener(|s,_,_|s.demo=true)))
            .child(label("ZIP · TAR · TAR.GZ · SLDPRT").font_size(12.).color(ThemeColor::TextMuted))
            .child(label(&self.status).key("status").font_size(13.))
            .child(row().fill_width().gap(6.)
                .child(button("Fit view").width(135.).key("fit").on_click(cx.listener(|s,_,_|s.reset=true)))
                .child(button(if self.wire{"Shaded"}else{"Wireframe"}).key("wire").on_click(cx.listener(|s,_,_|s.wire = !s.wire))))
            .child(button(if self.hidden{"Hide hidden Parts"}else{"Show hidden Parts"}).key("hidden").on_click(cx.listener(|s,_,_|s.hidden = !s.hidden)))
            .child(label(if query.is_empty() { format!("DOCUMENTS / {}", self.paths.len()) }
                else { format!("DOCUMENTS / {matches} OF {}", self.paths.len()) }).key("document-count").font_size(12.).color(ThemeColor::TextMuted))
            // The host overlays a native search input here for browser IME and accessibility.
            .child(label("").key("search-slot").fill_width().height(34.))
            .child(scroll_area(files).key("documents").fill_width().height(220.))
            .child(scroll_area(column().fill_width().gap(10.).child(configs).child(details)).fill_width().fill_height().min_height(0.))
            .child(label("SolidWorks XYZ · right-handed · assumed default\nCached physical values remain unverified.").font_size(11.).color(ThemeColor::TextMuted))
    }
}
/// RXUI sidebar hosted by an application-owned WebGPU canvas.
#[wasm_bindgen]
pub struct WebApp {
    graphics: GraphicsContext,
    target: Option<RenderTarget<'static>>,
    painter: UiPainter,
    runtime: Runtime,
    ui: Ui<Model>,
    logical: [f32; 2],
}
#[wasm_bindgen]
impl WebApp {
    pub async fn create(canvas: web_sys::HtmlCanvasElement) -> Result<Self, JsValue> {
        let (graphics, target) = GraphicsContext::with_surface(
            wgpu::SurfaceTarget::Canvas(canvas.clone()),
            SurfaceOptions::new(canvas.width(), canvas.height()).sample_count(4),
        )
        .await
        .map_err(error)?;
        let mut painter = UiPainter::new(&graphics);
        painter
            .fonts_mut()
            .load_font(include_bytes!("../assets/SourceSans3-Regular.otf"))
            .map_err(error)?;
        painter.set_sans_serif_family(Some("Source Sans 3".into()));
        let mut runtime = Runtime::new();
        runtime.begin_frame(Duration::ZERO);
        let root = runtime.update(|cx| {
            cx.new(|_| Model {
                status: "Open a CAD archive to begin.".into(),
                ..Default::default()
            })
        });
        let mut ui = Ui::new(&mut runtime, root).map_err(error)?;
        ui.set_theme(Theme::dark()).map_err(error)?;
        Ok(Self {
            graphics,
            target: Some(target),
            painter,
            runtime,
            ui,
            logical: [340., 800.],
        })
    }
    pub fn load_font(&mut self, bytes: &[u8]) -> Result<(), JsValue> {
        self.painter.fonts_mut().load_font(bytes).map_err(error)?;
        self.ui.invalidate_geometry();
        Ok(())
    }
    pub fn resize(&mut self, w: u32, h: u32) -> Result<(), JsValue> {
        self.target.as_mut().unwrap().resize(w, h).map_err(error)
    }
    pub fn pointer(&mut self, kind: &str, x: f32, y: f32) -> Result<(), JsValue> {
        self.prepare_input()?;
        let e = match kind {
            "down" => PointerEvent::Pressed([x, y]),
            "up" => PointerEvent::Released([x, y]),
            "cancel" => PointerEvent::Cancelled,
            "leave" => PointerEvent::Left,
            _ => PointerEvent::Moved([x, y]),
        };
        self.ui
            .pointer_with_text(&mut self.runtime, e, &mut self.painter, false)
            .map_err(error)?;
        self.runtime.flush().map_err(error)
    }
    pub fn key(&mut self, key: &str, shift: bool, pressed: bool) -> Result<(), JsValue> {
        self.prepare_input()?;
        let key = match key {
            "Tab" => rxui::KeyboardKey::Tab,
            "Enter" => rxui::KeyboardKey::Enter,
            "Escape" => rxui::KeyboardKey::Escape,
            "ArrowDown" => rxui::KeyboardKey::ArrowDown,
            "ArrowUp" => rxui::KeyboardKey::ArrowUp,
            "ArrowLeft" => rxui::KeyboardKey::ArrowLeft,
            "ArrowRight" => rxui::KeyboardKey::ArrowRight,
            " " => rxui::KeyboardKey::Space,
            _ => return Ok(()),
        };
        let default_key = key.clone();
        let result = self
            .ui
            .key(
                &mut self.runtime,
                rxui::KeyEvent {
                    key,
                    physical_key: rxui::PhysicalKey::Unidentified,
                    pressed,
                    repeat: false,
                    modifiers: Modifiers {
                        shift,
                        ..Default::default()
                    },
                },
            )
            .map_err(error)?;
        if pressed && !result.default_prevented {
            match default_key {
                rxui::KeyboardKey::Tab => {
                    self.ui.focus_next(shift);
                }
                rxui::KeyboardKey::Enter | rxui::KeyboardKey::Space => {
                    self.ui.activate_focused(&mut self.runtime).map_err(error)?;
                }
                _ => {}
            }
        }
        self.runtime.flush().map_err(error)
    }
    pub fn wheel(&mut self, x: f32, y: f32, dy: f32) -> Result<(), JsValue> {
        self.prepare_input()?;
        self.ui
            .wheel(&mut self.runtime, [x, y], [0., dy], Modifiers::default())
            .map_err(error)?;
        Ok(())
    }
    pub fn set_status(&mut self, status: String) {
        let entity = self.ui.entity().clone();
        self.runtime
            .update(|cx| entity.update(cx, |m, _| m.status = status));
    }
    pub fn clear(&mut self) {
        let entity = self.ui.entity().clone();
        self.runtime.update(|cx| {
            entity.update(cx, |m, _| {
                *m = Model {
                    status: "Reading archive…".into(),
                    ..Default::default()
                }
            })
        });
    }
    pub fn set_files(&mut self, json: &str) -> Result<(), JsValue> {
        let paths: Vec<String> = serde_json::from_str(json).map_err(error)?;
        let entity = self.ui.entity().clone();
        self.runtime.update(|cx| {
            entity.update(cx, |m, _| {
                m.paths = paths;
                m.occurrence_path = None;
                m.occurrence_config = None;
                m.selected = 0;
                m.config = 0;
                m.status = "Reading document…".into();
            })
        });
        Ok(())
    }
    /// Open a document or an embedded occurrence discovered by worker traversal.
    pub fn select_document(&mut self, path: String, configuration: Option<String>) {
        let entity = self.ui.entity().clone();
        self.runtime.update(|cx| {
            entity.update(cx, |m, _| {
                m.selected = m
                    .paths
                    .iter()
                    .position(|p| p == &path)
                    .unwrap_or(usize::MAX);
                m.occurrence_path = Some(path);
                m.occurrence_config = configuration;
                m.config = 0;
                m.configs.clear();
                m.status = "Loading document…".into();
            })
        });
    }
    /// Filter the document list without changing the selected document or scene.
    pub fn set_search(&mut self, query: String) {
        let entity = self.ui.entity().clone();
        self.runtime
            .update(|cx| entity.update(cx, |m, _| m.search = query));
    }
    pub fn show_scene(&mut self, json: &str) -> Result<(), JsValue> {
        let scene: serde_json::Value = serde_json::from_str(json).map_err(error)?;
        let configs = scene["configurations"]
            .as_array()
            .ok_or_else(|| error("invalid scene configs"))?
            .iter()
            .map(|v| v.as_str().unwrap_or("").to_owned())
            .collect();
        let info = scene["info"]
            .as_array()
            .ok_or_else(|| error("invalid scene details"))?
            .iter()
            .map(|v| v.as_str().unwrap_or("").to_owned())
            .collect();
        let issues = scene["diagnostics"]
            .as_array()
            .ok_or_else(|| error("invalid scene diagnostics"))?
            .iter()
            .map(|v| v.as_str().unwrap_or("").to_owned())
            .collect();
        let status = scene["status"].as_str().unwrap_or_default().to_owned();
        let selected_config = scene["selectedConfiguration"]
            .as_u64()
            .and_then(|i| usize::try_from(i).ok())
            .unwrap_or(0);
        let entity = self.ui.entity().clone();
        self.runtime.update(|cx| {
            entity.update(cx, |m, _| {
                m.configs = configs;
                m.config = selected_config;
                m.info = info;
                m.issues = issues;
                m.status = status;
            })
        });
        Ok(())
    }
    pub fn controls(&mut self) -> String {
        let entity = self.ui.entity().clone();
        self.runtime.update(|cx|entity.update(cx,|m,cx|{
            let unchanged=!(m.open || m.demo || m.reset);
            let out=serde_json::json!({"path":m.occurrence_path.as_ref().or_else(||m.paths.get(m.selected)),"configurationName":m.occurrence_config,"config":m.config,"hidden":m.hidden,"wire":m.wire,"fit":m.reset,"demo":m.demo,"open":m.open});m.open=false;m.demo=false;m.reset=false;if unchanged {cx.unchanged();}out.to_string()
        }))
    }
    pub fn animating(&self) -> bool {
        self.runtime.animation_frame_requested()
    }
    pub fn draw(&mut self, time: f64, scale: f32) -> Result<bool, JsValue> {
        if self.graphics.is_device_lost() {
            return Err(error("WebGPU device lost. Reload to recreate the viewer."));
        }
        let [w, h] = self.target.as_ref().unwrap().size();
        if w == 0 || h == 0 {
            return Ok(false);
        }
        self.logical = [w as f32 / scale, h as f32 / scale];
        self.runtime
            .begin_frame(Duration::from_secs_f64(time / 1000.));
        self.runtime.flush().map_err(error)?;
        self.ui
            .prepare(&mut self.runtime, self.logical, &mut self.painter)
            .map_err(error)?;
        self.painter
            .prepare(
                &self.ui,
                &self.target.as_ref().unwrap().render_format(),
                scale,
            )
            .map_err(error)?;
        let mut frame = match self.target.as_mut().unwrap().begin_frame() {
            Ok(f) => f,
            Err(FrameError::Retry | FrameError::Suspended) => return Ok(false),
            Err(e) => return Err(error(e)),
        };
        self.painter
            .compose(&self.ui, &mut frame, scale, |frame, ui| {
                let mut pass = frame
                    .render_pass()
                    .clear_color(wgpu::Color {
                        r: 0.035,
                        g: 0.046,
                        b: 0.06,
                        a: 1.,
                    })
                    .begin()?;
                ui.paint(&mut pass)
            })
            .map_err(error)?;
        frame.finish().map_err(error)?;
        Ok(true)
    }
    /// Logical bounds for the browser-native search field over the RXUI list.
    pub fn search_bounds(&self) -> Vec<f32> {
        let key = Key::from("search-slot");
        self.ui
            .elements()
            .find(|e| e.key == Some(&key))
            .map(|e| vec![e.bounds.x, e.bounds.y, e.bounds.width, e.bounds.height])
            .unwrap_or_default()
    }
    /// Inspector snapshot used by browser tests; no raw CAD is returned.
    pub fn snapshot(&mut self) -> String {
        let _ = self.prepare_input();
        let entity = self.ui.entity().clone();
        let state=self.runtime.update(|cx|{let m=entity.read(cx);serde_json::json!({"documents":m.paths.len(),"selected":m.occurrence_path.as_ref().or_else(||m.paths.get(m.selected)),"status":m.status,"info":m.info,"diagnostics":m.issues})});
        let elements:Vec<_>=self.ui.elements().map(|e|serde_json::json!({"key":e.key.map(|k|format!("{k:?}")),"text":e.text,"bounds":[e.bounds.x,e.bounds.y,e.bounds.width,e.bounds.height]})).collect();
        serde_json::json!({"state":state,"elements":elements}).to_string()
    }
}
impl WebApp {
    fn prepare_input(&mut self) -> Result<(), JsValue> {
        if self.ui.needs_prepare(&self.runtime).map_err(error)? {
            self.runtime.flush().map_err(error)?;
            self.ui
                .prepare(&mut self.runtime, self.logical, &mut self.painter)
                .map_err(error)?;
        }
        Ok(())
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = self, js_name = reportImportProgress)]
    fn report_import_progress(completed: f64, total: f64, path: &str);
}
/// Worker-local parser state; never constructs a graphics device or reads disk.
#[wasm_bindgen]
pub struct ArchiveProject {
    project: Project,
    scene: Scene,
}
#[wasm_bindgen]
impl ArchiveProject {
    #[wasm_bindgen(constructor)]
    pub fn new(bytes: &[u8], filename: &str) -> Result<Self, JsValue> {
        let lower = filename.to_ascii_lowercase();
        let files = if [".sldprt", ".sldasm", ".slddrw"]
            .iter()
            .any(|s| lower.ends_with(s))
        {
            let mut files = sw_rs::archive::FileSet::default();
            let name = sw_rs::archive::normalize_path(filename).map_err(error)?;
            files.documents.insert(name, bytes.to_vec().into());
            files
        } else {
            sw_rs::archive::FileSet::from_archive_with_progress(
                bytes,
                filename,
                sw_rs::archive::ArchiveLimits::unlimited(),
                |p| {
                    report_import_progress(
                        p.completed_entries as f64,
                        p.total_entries.map(|n| n as f64).unwrap_or(0.),
                        p.path,
                    )
                },
            )
            .map_err(error)?
        };
        Ok(Self {
            project: Project::new(files),
            scene: Scene::default(),
        })
    }
    /// On-demand source inventory; never changes scene selection or mesh buffers.
    pub fn inventory(
        &mut self,
        path: &str,
        stream: Option<String>,
        offset: Option<u32>,
    ) -> Result<String, JsValue> {
        self.project
            .inventory(path, stream.as_deref(), offset.unwrap_or(0) as usize)
            .map(|value| value.to_string())
            .map_err(error)
    }
    /// Release streams retained by the optional inventory inspector.
    pub fn clear_inventory(&mut self) {
        self.project.clear_inventory();
    }
    pub fn paths(&self) -> String {
        let mut paths: Vec<_> = self.project.files.documents.keys().collect();
        paths.sort_by_key(|p| {
            (
                !p.to_ascii_lowercase().ends_with(".sldasm"),
                p.matches('/').count(),
                std::cmp::Reverse(if p.to_ascii_lowercase().ends_with(".sldasm") {
                    self.project.files.documents[*p].len()
                } else {
                    0
                }),
                p.to_lowercase(),
            )
        });
        serde_json::to_string(&paths).unwrap()
    }
    pub fn scene(
        &mut self,
        path: &str,
        config: usize,
        hidden: bool,
        configuration_name: Option<String>,
    ) -> String {
        let config = configuration_name
            .as_deref()
            .map(|name| self.project.configuration_index(path, name))
            .unwrap_or(config);
        self.scene = self.project.scene(path, config, hidden);
        let triangles = self.scene.vertices.len() / 27;
        let mut info = vec![
            path.to_owned(),
            format!(
                "{} · {} configuration(s)",
                self.scene.metadata["doc_type"]
                    .as_str()
                    .unwrap_or("unknown"),
                self.scene.configurations.len()
            ),
            format!(
                "{} occurrences · {triangles} triangles",
                self.scene.occurrences
            ),
        ];
        if self.scene.metadata["coordinate_frame"].is_object() {
            info.push("Model frame: SolidWorks XYZ · right-handed".into());
            info.push("Front: XY · +Y up · +Z toward observer (default convention)".into());
            info.push("Geometry: metres in viewer · original document origin retained".into());
        }
        if !self.scene.materials.is_empty() {
            info.push(format!(
                "{} saved material preset(s) · approximate appearances",
                self.scene.materials.len()
            ));
        }
        if let Some(mass) =
            self.scene.metadata["configurations"][config]["mass_properties"]["mass"].as_f64()
        {
            info.push(format!("Cached mass token: {mass:.5} (units unverified)"));
        }
        info.extend(crate::material_info::lines(&self.scene.metadata, config));
        if let Some(index) = self.scene.metadata["configurations"][config]["index"].as_u64()
            && let Some(streams) = self.scene.metadata["mates"]["configurations"].as_array()
        {
            for stream in streams
                .iter()
                .filter(|s| s["configuration_index"].as_u64() == Some(index))
            {
                let recognized = stream["records"].as_array().map_or(0, Vec::len);
                let declared = stream["declared_count"]
                    .as_u64()
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "unknown".into());
                info.push(format!(
                    "Saved mates: {recognized}/{declared} headers · partial relationships"
                ));
            }
        }
        if let Some(props) = self.scene.metadata["global_properties"].as_object() {
            for (key, value) in props.iter().take(8) {
                info.push(
                    format!("{key}: {}", value.as_str().unwrap_or_default())
                        .chars()
                        .take(140)
                        .collect(),
                );
            }
        }
        if self.scene.missing_references > 0 {
            info.insert(
                0,
                format!(
                    "Partial assembly · {} missing reference(s) skipped",
                    self.scene.missing_references
                ),
            );
        }
        let status = if self.scene.missing_references > 0 {
            format!(
                "Partial assembly · {triangles} triangles · {} missing reference(s) · {} issue(s)",
                self.scene.missing_references,
                self.scene.diagnostics.len()
            )
        } else if triangles > 0 {
            format!(
                "{triangles} triangles · {} issue(s)",
                self.scene.diagnostics.len()
            )
        } else {
            "Metadata only · no supported mesh".into()
        };
        serde_json::json!({"metadata":self.scene.metadata,"hierarchy":self.scene.hierarchy,"presentations":self.scene.presentations,"materials":self.scene.materials,"batches":self.scene.batches,"configurations":self.scene.configurations,"selectedConfiguration":config,"info":info,"diagnostics":self.scene.diagnostics,"status":status,"triangles":triangles,"occurrences":self.scene.occurrences,"missing_references":self.scene.missing_references}).to_string()
    }
    pub fn take_vertices(&mut self) -> Vec<f32> {
        std::mem::take(&mut self.scene.vertices)
    }
}
