// SPDX-License-Identifier: MIT
// Copyright (c) 2026 hxyulin <hxyulin@proton.me>
//! Native workbench using RXUI's system theme and standard controls.
mod viewport;

use rxui::{Element, prelude::*, taffy::prelude::AlignItems};
use std::{collections::BTreeSet, path::PathBuf, sync::Arc};
use sw_rs::archive::{ArchiveLimits, FileSet};
use sw_viewer::project::{Project, Scene};
use viewport::{Camera, Mesh, Viewport};

struct Workbench {
    project: Option<Project>,
    scene: Scene,
    mesh: Arc<Mesh>,
    paths: Vec<String>,
    path: String,
    configuration: usize,
    search: String,
    status: String,
    tree: TreeState,
    tree_scroll: ScrollHandle,
    camera: Camera,
    drag: Option<([f32; 2], Camera)>,
    show_hidden: bool,
    material_colors: bool,
    task: Option<Task>,
    dialog: Option<DialogTask>,
}

impl Workbench {
    fn new() -> Self {
        Self {
            project: None,
            scene: Scene::default(),
            mesh: Arc::new(Mesh::default()),
            paths: Vec::new(),
            path: String::new(),
            configuration: 0,
            search: String::new(),
            status: "Open a SolidWorks file or project archive.".into(),
            tree: TreeState::new(TreeData::new(Vec::new()).unwrap(), SelectionMode::Single),
            tree_scroll: ScrollHandle::new(),
            camera: Camera::default(),
            drag: None,
            show_hidden: false,
            material_colors: true,
            task: None,
            dialog: None,
        }
    }

    fn open(&mut self, cx: &mut Context<'_, Self>) {
        let options = FileDialog::new()
            .title("Open SolidWorks file or project")
            .filter(
                "SolidWorks and archives",
                ["sldprt", "sldasm", "slddrw", "zip", "tar", "gz", "tgz"],
            );
        match cx.pick_file(options, |this, result, cx| {
            this.dialog = None;
            match result {
                Ok(Some(path)) => this.load(path, cx),
                Ok(None) => {}
                Err(error) => this.status = error.to_string(),
            }
        }) {
            Ok(dialog) => self.dialog = Some(dialog),
            Err(error) => self.status = error.to_string(),
        }
    }

    fn load(&mut self, path: PathBuf, cx: &mut Context<'_, Self>) {
        self.status = "Reading project…".into();
        self.task = Some(cx.spawn_blocking(
            move || load_project(path),
            |this, result, _| {
                this.task = None;
                match result.map_err(|e| e.to_string()).and_then(|r| r) {
                    Ok((project, scene, paths, path)) => {
                        this.project = Some(project);
                        this.paths = paths;
                        this.path = path;
                        this.configuration = 0;
                        this.search.clear();
                        this.show_hidden = false;
                        this.accept_scene(scene);
                    }
                    Err(error) => this.status = error,
                }
            },
        ));
    }

    fn select(&mut self, path: String, config: usize, cx: &mut Context<'_, Self>) {
        let Some(mut project) = self.project.take() else {
            return;
        };
        self.path = path.clone();
        self.configuration = config;
        self.status = "Loading document…".into();
        let hidden = self.show_hidden;
        self.task = Some(cx.spawn_blocking(
            move || {
                let scene = project.scene(&path, config, hidden);
                (project, scene)
            },
            |this, result, _| {
                this.task = None;
                match result {
                    Ok((project, scene)) => {
                        this.project = Some(project);
                        this.accept_scene(scene);
                    }
                    Err(error) => this.status = error.to_string(),
                }
            },
        ));
    }

    fn accept_scene(&mut self, scene: Scene) {
        self.mesh = Arc::new(Mesh::from_scene(&scene));
        self.camera = Camera::default();
        let parents: BTreeSet<_> = scene
            .hierarchy
            .iter()
            .filter_map(|n| n["parent"].as_u64())
            .collect();
        let nodes = scene
            .hierarchy
            .iter()
            .enumerate()
            .map(|(i, n)| {
                let status = n["status"].as_str().unwrap_or("unknown");
                let name = n["name"].as_str().unwrap_or("Unnamed");
                TreeNode {
                    key: Key::from(i.to_string()),
                    parent: n["parent"].as_u64().map(|p| Key::from(p.to_string())),
                    label: if status == "available" {
                        name.into()
                    } else {
                        format!("{name} · {status}")
                    },
                    branch: parents.contains(&(i as u64)),
                }
            })
            .collect();
        self.tree = TreeState::new(
            TreeData::new(nodes).expect("parser preorder hierarchy"),
            SelectionMode::Single,
        );
        self.tree.set_expanded([Key::from("0")]);
        self.status = if scene.vertices.is_empty() {
            "Metadata only · no supported mesh".into()
        } else {
            format!(
                "{} triangles · {} occurrences · {} issues",
                scene.vertices.len() / 27,
                scene.occurrences,
                scene.diagnostics.len()
            )
        };
        self.scene = scene;
    }

    fn inspector(&self) -> Element {
        let mut panel = column().gap(16.).fill_width();
        panel = panel.child(label("Properties").text_role(TextRole::Heading));
        if self.path.is_empty() {
            return panel
                .child(label("Select a document to inspect it.").color(ThemeColor::TextMuted));
        }
        let meta = &self.scene.metadata;
        for (name, value) in [
            (
                "Document",
                self.path
                    .rsplit('/')
                    .next()
                    .unwrap_or(&self.path)
                    .to_owned(),
            ),
            (
                "Type",
                meta["doc_type"].as_str().unwrap_or("Unknown").to_owned(),
            ),
            (
                "Configuration",
                self.scene
                    .configurations
                    .get(self.configuration)
                    .cloned()
                    .unwrap_or_default(),
            ),
            ("Triangles", (self.scene.vertices.len() / 27).to_string()),
            ("Occurrences", self.scene.occurrences.to_string()),
        ] {
            panel = panel.child(
                column()
                    .gap(4.)
                    .child(
                        label(name)
                            .text_role(TextRole::Caption)
                            .color(ThemeColor::TextMuted),
                    )
                    .child(label(value)),
            );
        }
        if let Some(key) = self.tree.active() {
            let selected = self
                .scene
                .hierarchy
                .iter()
                .enumerate()
                .find(|(i, _)| Key::from(i.to_string()) == *key);
            if let Some((_, node)) = selected {
                panel = panel.child(label("Occurrence").text_role(TextRole::Heading));
                for field in ["name", "configuration", "status"] {
                    if let Some(value) = node[field].as_str().filter(|v| !v.is_empty()) {
                        panel = panel.child(label(value));
                    }
                }
            }
        }
        if let Some(props) = meta["global_properties"]
            .as_object()
            .filter(|p| !p.is_empty())
        {
            panel = panel.child(label("Saved properties").text_role(TextRole::Heading));
            for (key, value) in props {
                panel = panel.child(
                    column()
                        .gap(4.)
                        .child(
                            label(key)
                                .text_role(TextRole::Caption)
                                .color(ThemeColor::TextMuted),
                        )
                        .child(label(value.as_str().unwrap_or_default())),
                );
            }
        }
        if !self.scene.diagnostics.is_empty() {
            panel = panel.child(label("Issues").text_role(TextRole::Heading));
            for issue in &self.scene.diagnostics {
                panel = panel.child(label(issue));
            }
        }
        panel
    }
}

impl View for Workbench {
    fn view(&self, cx: &mut ViewContext<'_, Self>) -> impl IntoElement {
        let busy = self.task.is_some();
        let query = self.search.trim().to_lowercase();
        let mut documents = column().gap(2.).fill_width();
        for path in self
            .paths
            .iter()
            .filter(|p| p.to_lowercase().contains(&query))
        {
            let selected = path.clone();
            documents = documents.child(
                button(path.rsplit('/').next().unwrap_or(path))
                    .key(path.clone())
                    .list_row(path == &self.path)
                    .disabled(busy)
                    .on_click(cx.listener(move |this, _, cx| this.select(selected.clone(), 0, cx))),
            );
        }
        let tree_change = cx.listener(|this: &mut Self, proposal: &TreeProposal, cx| {
            if let Err(error) = proposal.accept(&mut this.tree, &this.tree_scroll, 30., cx) {
                this.status = error.to_string();
            }
        });
        let tree = virtual_tree(
            &self.tree,
            30.,
            &self.tree_scroll,
            cx,
            tree_change,
            |_, node, _| label(&node.label),
        );
        let sidebar = column()
            .key("sidebar")
            .width(260.)
            .min_width(220.)
            .fill_height()
            .min_height(0.)
            .padding(16.)
            .gap(12.)
            .background(ThemeColor::Surface)
            .child(label("Documents").text_role(TextRole::Heading))
            .child(
                text_input(&self.search)
                    .key("search")
                    .placeholder("Search documents…")
                    .fill_width()
                    .accessibility_label("Search documents")
                    .on_change(cx.listener(|this, event: &TextChangeEvent, _| {
                        this.search = event.value.clone()
                    })),
            )
            .child(scroll_area(documents).fill_width().height(180.))
            .child(label("Occurrences").text_role(TextRole::Heading))
            .child(
                tree.key("occurrences")
                    .fill_width()
                    .flex_grow(1.)
                    .flex_basis(0.)
                    .min_height(0.),
            );
        let mut views = row().gap(4.).align_items(AlignItems::CENTER);
        for (name, yaw, pitch) in [
            ("Front", 0., 0.),
            ("Top", 0., std::f32::consts::FRAC_PI_2),
            ("Right", std::f32::consts::FRAC_PI_2, 0.),
            ("Isometric", 0.65, 0.45),
        ] {
            views = views.child(
                button(name)
                    .variant(ButtonVariant::Quiet)
                    .on_click(cx.listener(move |this, _, _| {
                        this.camera.yaw = yaw;
                        this.camera.pitch = pitch;
                    })),
            );
        }
        let viewport = rxui::custom(Viewport {
            mesh: self.mesh.clone(),
            camera: self.camera,
            material_colors: self.material_colors,
            background: cx.theme().color(ThemeColor::Background),
        })
        .key("viewport")
        .fill_width()
        .flex_grow(1.)
        .flex_basis(0.)
        .min_height(0.)
        .accessibility_label("3D model. Drag to orbit, Shift-drag to pan, scroll to zoom.")
        .on_pointer_down(cx.listener(|this, event: &rxui::PointerInput, _| {
            this.drag = Some((event.position, this.camera));
            event.capture_pointer();
        }))
        .on_pointer_move(cx.listener(|this, event: &rxui::PointerInput, _| {
            if let Some((start, camera)) = this.drag {
                let delta = [event.position[0] - start[0], event.position[1] - start[1]];
                if event.modifiers.shift || event.buttons.contains(rxui::PointerButton::Secondary) {
                    this.camera.pan = [
                        camera.pan[0] + delta[0] / event.bounds.height * 2.,
                        camera.pan[1] - delta[1] / event.bounds.height * 2.,
                    ];
                } else {
                    this.camera.yaw = camera.yaw + delta[0] * 0.008;
                    this.camera.pitch = (camera.pitch + delta[1] * 0.008).clamp(-1.57, 1.57);
                }
            }
        }))
        .on_pointer_up(cx.listener(|this, event: &rxui::PointerInput, _| {
            this.drag = None;
            event.release_pointer();
        }))
        .on_pointer_cancel(cx.listener(|this, _, _| this.drag = None))
        .on_wheel(cx.listener(|this, event: &rxui::WheelInput, _| {
            this.camera.zoom =
                (this.camera.zoom * (-event.delta[1] * 0.001).exp()).clamp(0.05, 50.);
            event.prevent_default();
        }));
        let center = column()
            .flex_grow(1.)
            .flex_basis(0.)
            .fill_height()
            .min_width(0.)
            .min_height(0.)
            .child(
                row()
                    .padding(8.)
                    .gap(12.)
                    .align_items(AlignItems::CENTER)
                    .child(views)
                    .child(
                        button("Fit")
                            .key("fit")
                            .on_click(cx.listener(|this, _, _| this.camera = Camera::default())),
                    )
                    .child(
                        button(if self.material_colors {
                            "Material colors"
                        } else {
                            "Diagnostic colors"
                        })
                        .key("colors")
                        .variant(ButtonVariant::Quiet)
                        .on_click(
                            cx.listener(|this, _, _| this.material_colors = !this.material_colors),
                        ),
                    ),
            )
            .child(viewport)
            .child(
                label(if self.mesh.vertices.is_empty() {
                    "Open a model to begin"
                } else {
                    "Drag to orbit · Shift-drag to pan · Scroll to zoom"
                })
                .text_role(TextRole::Caption)
                .color(ThemeColor::TextMuted)
                .padding(12.),
            );
        let options = self
            .scene
            .configurations
            .iter()
            .enumerate()
            .map(|(i, name)| choice(i.to_string(), name))
            .collect::<Vec<_>>();
        let configuration = select(cx)
            .key("configuration")
            .selected(self.configuration.to_string())
            .options(options)
            .width(180.)
            .disabled(busy || self.scene.configurations.is_empty())
            .on_change(cx.listener(|this, event: &ChoiceEvent, cx| {
                if let Some(config) = (0..this.scene.configurations.len())
                    .find(|i| Key::from(i.to_string()) == event.key)
                {
                    this.select(this.path.clone(), config, cx);
                }
            }));
        column()
            .fill_width()
            .fill_height()
            .min_height(0.)
            .child(
                row()
                    .padding(12.)
                    .gap(16.)
                    .align_items(AlignItems::CENTER)
                    .child(label("SW Explorer").text_role(TextRole::Heading))
                    .child(
                        button("Open…")
                            .disabled(self.dialog.is_some())
                            .variant(ButtonVariant::Primary)
                            .on_click(cx.listener(|this, _, cx| this.open(cx))),
                    )
                    .child(configuration)
                    .child(
                        button(if self.show_hidden {
                            "Hide hidden"
                        } else {
                            "Show hidden"
                        })
                        .disabled(busy || self.path.is_empty())
                        .on_click(cx.listener(|this, _, cx| {
                            this.show_hidden = !this.show_hidden;
                            this.select(this.path.clone(), this.configuration, cx);
                        })),
                    ),
            )
            .child(
                row()
                    .fill_width()
                    .flex_grow(1.)
                    .flex_basis(0.)
                    .min_height(0.)
                    .child(sidebar)
                    .child(center)
                    .child(
                        column()
                            .key("inspector")
                            .width(280.)
                            .fill_height()
                            .min_height(0.)
                            .padding(20.)
                            .background(ThemeColor::Surface)
                            .child(scroll_area(self.inspector()).fill_width().fill_height()),
                    ),
            )
            .child(
                label(&self.status)
                    .key("status")
                    .padding(10.)
                    .text_role(TextRole::Caption)
                    .color(ThemeColor::TextMuted),
            )
    }
}

type LoadedProject = (Project, Scene, Vec<String>, String);
fn load_project(path: PathBuf) -> Result<LoadedProject, String> {
    let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
    let filename = path
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or("Invalid filename")?;
    let lower = filename.to_ascii_lowercase();
    let files = if [".sldprt", ".sldasm", ".slddrw"]
        .iter()
        .any(|ext| lower.ends_with(ext))
    {
        let mut files = FileSet::default();
        files.documents.insert(filename.into(), bytes.into());
        files
    } else {
        FileSet::from_archive(&bytes, filename, ArchiveLimits::unlimited())
            .map_err(|e| e.to_string())?
    };
    let paths: Vec<_> = files.documents.keys().cloned().collect();
    let selected = paths
        .iter()
        .find(|p| p.to_ascii_lowercase().ends_with(".sldasm"))
        .or_else(|| paths.first())
        .cloned()
        .ok_or("No SolidWorks documents found")?;
    let mut project = Project::new(files);
    let scene = project.scene(&selected, 0, false);
    Ok((project, scene, paths, selected))
}

pub fn run() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args_os().skip(1);
    let path = args.next();
    if path.as_deref().is_some_and(|p| p == "--help" || p == "-h") {
        println!(
            "Usage: sw-viewer [FILE]\n\nOpen a SolidWorks file or ZIP/TAR/TAR.GZ project in the native RXUI viewer."
        );
        return Ok(());
    }
    if args.next().is_some() {
        return Err("Usage: sw-viewer [FILE]".into());
    }
    let path = path.map(PathBuf::from);
    Application::new()
        .error(|error, _| {
            eprintln!("Native viewer: {error}");
            rxui::host::ErrorAction::Exit
        })
        .run(|cx| {
            let model = cx.new(|_| Workbench::new());
            cx.open_window(
                WindowOptions::new()
                    .title("SW Explorer")
                    .size(1440., 900.)
                    .min_size(1050., 650.),
                model.clone(),
            )?;
            if let Some(path) = path {
                model.update(cx, |this, cx| this.load(path, cx));
            }
            Ok(())
        })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires a native graphics adapter; run with --features native -- --ignored"]
    fn native_workbench_renders_resizes_and_routes_controls() {
        let graphics = pollster::block_on(rxui::drawing::GraphicsContext::headless()).unwrap();
        let mut model = Workbench::new();
        model.path = "sample.SLDPRT".into();
        model.paths = vec![model.path.clone()];
        model.accept_scene(Scene {
            vertices: vec![
                -1., -1., 0., 0., 0., 1., 0.2, 0.7, 0.8, 1., -1., 0., 0., 0., 1., 0.2, 0.7, 0.8,
                0., 1., 0., 0., 0., 1., 0.2, 0.7, 0.8,
            ],
            hierarchy: vec![
                serde_json::json!({"parent":null,"name":"sample.SLDPRT","status":"available"}),
            ],
            ..Default::default()
        });
        let mut ui = rxui::TestUi::new(&graphics, [1440, 900], model).unwrap();
        ui.painter_mut().fonts_mut().load_system_fonts();
        for size in [[1440, 900], [1050, 650]] {
            ui.resize(size);
            let pixels = ui.render().unwrap();
            assert_eq!(pixels.len(), size[0] as usize * size[1] as usize * 4);
            let sidebar = ui.element("sidebar").unwrap().unwrap().bounds;
            let viewport = ui.element("viewport").unwrap().unwrap().bounds;
            let inspector = ui.element("inspector").unwrap().unwrap().bounds;
            let status = ui.element("status").unwrap().unwrap().bounds;
            assert!(sidebar.x + sidebar.width <= viewport.x);
            assert!(viewport.x + viewport.width <= inspector.x + 1.);
            assert!(inspector.x + inspector.width <= size[0] as f32 + 1.);
            assert!(status.y + status.height <= size[1] as f32 + 1.);
            assert!(viewport.width > 300. && viewport.height > 400.);
        }
        assert!(ui.click("colors").unwrap());
        assert!(!ui.read(|m| m.material_colors));
        let viewport = ui.element("viewport").unwrap().unwrap().bounds;
        ui.wheel([viewport.x + 50., viewport.y + 50.], [0., -120.])
            .unwrap();
        assert!(ui.read(|m| m.camera.zoom) > 1.);
        assert!(ui.click("fit").unwrap());
        assert_eq!(ui.read(|m| m.camera.zoom), 1.);
        ui.render().unwrap();
    }
}
