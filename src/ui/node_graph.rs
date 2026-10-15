//! Node graph widget - for Fusion and Color pages

use crate::core::id::{GradeNodeId, TrackItemId};
use crate::core::math::{Point2, Rect};
use crate::ui::layout::*;
use crate::ui::theme::Theme;
use crate::ui::widgets::*;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

#[derive(Debug, Clone)]
pub struct Node {
    pub id: GradeNodeId,
    pub name: String,
    pub node_type: NodeType,
    pub position: Point2,
    pub size: (f32, f32),
    pub inputs: Vec<NodePort>,
    pub outputs: Vec<NodePort>,
    pub enabled: bool,
    pub selected: bool,
    pub color: [f32; 4],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeType {
    // Color
    Serial,
    Parallel,
    Layer,
    Outside,
    // Fusion
    Loader,
    Saver,
    Merge,
    Transform,
    Blur,
    ColorCorrect,
    Text,
    Background,
    Custom,
}

#[derive(Debug, Clone)]
pub struct NodePort {
    pub id: String,
    pub name: String,
    pub port_type: PortType,
    pub data_type: PortDataType,
    pub connected: Option<(GradeNodeId, String)>, // (node_id, port_id)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortType {
    Input,
    Output,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortDataType {
    Image,
    Mask,
    Float,
    Int,
    String,
    Any,
}

#[derive(Clone)]
pub struct NodeGraph {
    nodes: HashMap<GradeNodeId, Node>,
    connections: Vec<NodeConnection>,
    selected_nodes: Vec<GradeNodeId>,
    pan: Point2,
    zoom: f32,
    grid_size: f32,
    show_grid: bool,
    rect: Rect,
    drag_state: Option<NodeDragState>,
    connection_drag: Option<ConnectionDrag>,
}

#[derive(Debug, Clone)]
struct NodeConnection {
    pub from_node: GradeNodeId,
    pub from_port: String,
    pub to_node: GradeNodeId,
    pub to_port: String,
}

#[derive(Debug, Clone)]
struct NodeDragState {
    node_id: GradeNodeId,
    start_pos: Point2,
    start_mouse: Point2,
    multi_select: bool,
}

#[derive(Debug, Clone)]
struct ConnectionDrag {
    from_node: GradeNodeId,
    from_port: String,
    start_pos: Point2,
    current_pos: Point2,
    is_input: bool,
}

impl NodeGraph {
    pub fn new() -> Self {
        Self {
            nodes: HashMap::new(),
            connections: Vec::new(),
            selected_nodes: Vec::new(),
            pan: Point2::new(0.0, 0.0),
            zoom: 1.0,
            grid_size: 50.0,
            show_grid: true,
            rect: Rect::new(0.0, 0.0, 0.0, 0.0),
            drag_state: None,
            connection_drag: None,
        }
    }

    pub fn add_node(&mut self, node_type: NodeType, position: Point2) -> GradeNodeId {
        let id = GradeNodeId::generate();
        let name = format!("{:?} {}", node_type, self.nodes.len() + 1);

        let (inputs, outputs) = Self::default_ports(node_type);
        let color = Self::color_for_type(node_type);

        let node = Node {
            id,
            name,
            node_type,
            position,
            size: (150.0, 100.0),
            inputs,
            outputs,
            enabled: true,
            selected: false,
            color,
        };

        self.nodes.insert(id, node);
        id
    }

    fn default_ports(node_type: NodeType) -> (Vec<NodePort>, Vec<NodePort>) {
        match node_type {
            NodeType::Loader => (
                vec![],
                vec![NodePort {
                    id: "out".into(),
                    name: "Output".into(),
                    port_type: PortType::Output,
                    data_type: PortDataType::Image,
                    connected: None,
                }],
            ),
            NodeType::Saver => (
                vec![NodePort {
                    id: "in".into(),
                    name: "Input".into(),
                    port_type: PortType::Input,
                    data_type: PortDataType::Image,
                    connected: None,
                }],
                vec![],
            ),
            NodeType::Merge => (
                vec![
                    NodePort {
                        id: "bg".into(),
                        name: "Background".into(),
                        port_type: PortType::Input,
                        data_type: PortDataType::Image,
                        connected: None,
                    },
                    NodePort {
                        id: "fg".into(),
                        name: "Foreground".into(),
                        port_type: PortType::Input,
                        data_type: PortDataType::Image,
                        connected: None,
                    },
                    NodePort {
                        id: "mask".into(),
                        name: "Mask".into(),
                        port_type: PortType::Input,
                        data_type: PortDataType::Mask,
                        connected: None,
                    },
                ],
                vec![NodePort {
                    id: "out".into(),
                    name: "Output".into(),
                    port_type: PortType::Output,
                    data_type: PortDataType::Image,
                    connected: None,
                }],
            ),
            NodeType::Transform => (
                vec![NodePort {
                    id: "in".into(),
                    name: "Input".into(),
                    port_type: PortType::Input,
                    data_type: PortDataType::Image,
                    connected: None,
                }],
                vec![NodePort {
                    id: "out".into(),
                    name: "Output".into(),
                    port_type: PortType::Output,
                    data_type: PortDataType::Image,
                    connected: None,
                }],
            ),
            NodeType::Blur => (
                vec![NodePort {
                    id: "in".into(),
                    name: "Input".into(),
                    port_type: PortType::Input,
                    data_type: PortDataType::Image,
                    connected: None,
                }],
                vec![NodePort {
                    id: "out".into(),
                    name: "Output".into(),
                    port_type: PortType::Output,
                    data_type: PortDataType::Image,
                    connected: None,
                }],
            ),
            _ => (
                vec![NodePort {
                    id: "in".into(),
                    name: "Input".into(),
                    port_type: PortType::Input,
                    data_type: PortDataType::Any,
                    connected: None,
                }],
                vec![NodePort {
                    id: "out".into(),
                    name: "Output".into(),
                    port_type: PortType::Output,
                    data_type: PortDataType::Any,
                    connected: None,
                }],
            ),
        }
    }

    fn color_for_type(node_type: NodeType) -> [f32; 4] {
        match node_type {
            NodeType::Loader => [0.2, 0.6, 1.0, 1.0],
            NodeType::Saver => [1.0, 0.4, 0.2, 1.0],
            NodeType::Merge => [0.8, 0.2, 0.8, 1.0],
            NodeType::Transform => [0.2, 0.8, 0.4, 1.0],
            NodeType::Blur => [0.6, 0.4, 0.8, 1.0],
            NodeType::ColorCorrect => [1.0, 0.8, 0.2, 1.0],
            NodeType::Text => [0.4, 0.8, 0.8, 1.0],
            NodeType::Background => [0.5, 0.5, 0.5, 1.0],
            _ => [0.4, 0.4, 0.4, 1.0],
        }
    }

    pub fn remove_node(&mut self, node_id: GradeNodeId) {
        // Remove connections
        self.connections
            .retain(|c| c.from_node != node_id && c.to_node != node_id);
        // Remove node
        self.nodes.remove(&node_id);
        self.selected_nodes.retain(|&id| id != node_id);
    }

    pub fn connect(
        &mut self,
        from_node: GradeNodeId,
        from_port: String,
        to_node: GradeNodeId,
        to_port: String,
    ) -> bool {
        // Check if connection already exists
        if self.connections.iter().any(|c| {
            c.from_node == from_node
                && c.from_port == from_port
                && c.to_node == to_node
                && c.to_port == to_port
        }) {
            return false;
        }

        // Check for cycles (simplified)
        if self.would_create_cycle(from_node, to_node) {
            return false;
        }

        // Update port connections
        if let Some(from) = self.nodes.get_mut(&from_node) {
            if let Some(port) = from.outputs.iter_mut().find(|p| p.id == from_port) {
                port.connected = Some((to_node, to_port.clone()));
            }
        }
        if let Some(to) = self.nodes.get_mut(&to_node) {
            if let Some(port) = to.inputs.iter_mut().find(|p| p.id == to_port) {
                port.connected = Some((from_node, from_port.clone()));
            }
        }

        self.connections.push(NodeConnection {
            from_node,
            from_port,
            to_node,
            to_port,
        });
        true
    }

    fn would_create_cycle(&self, from: GradeNodeId, to: GradeNodeId) -> bool {
        // Simple cycle detection - check if 'to' can reach 'from'
        let mut visited = std::collections::HashSet::new();
        let mut stack = vec![to];

        while let Some(current) = stack.pop() {
            if current == from {
                return true;
            }
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current);

            // Find nodes that current connects to
            for conn in &self.connections {
                if conn.from_node == current {
                    stack.push(conn.to_node);
                }
            }
        }
        false
    }

    pub fn select_node(&mut self, node_id: GradeNodeId, add: bool) {
        if !add {
            self.selected_nodes.clear();
        }
        if !self.selected_nodes.contains(&node_id) {
            self.selected_nodes.push(node_id);
            if let Some(node) = self.nodes.get_mut(&node_id) {
                node.selected = true;
            }
        }
    }

    pub fn deselect_all(&mut self) {
        for id in &self.selected_nodes {
            if let Some(node) = self.nodes.get_mut(id) {
                node.selected = false;
            }
        }
        self.selected_nodes.clear();
    }

    pub fn set_pan(&mut self, pan: Point2) {
        self.pan = pan;
    }
    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = zoom.clamp(0.1, 5.0);
    }
    pub fn screen_to_graph(&self, screen: Point2) -> Point2 {
        Point2::new(
            (screen.x - self.pan.x) / self.zoom,
            (screen.y - self.pan.y) / self.zoom,
        )
    }
    pub fn graph_to_screen(&self, graph: Point2) -> Point2 {
        Point2::new(
            graph.x * self.zoom + self.pan.x,
            graph.y * self.zoom + self.pan.y,
        )
    }
}

impl NodeGraph {
    fn draw_grid(&self, ctx: &DrawContext, rect: Rect) {
        let grid = self.grid_size * self.zoom;
        let start_x = (rect.x - self.pan.x) / grid * grid + self.pan.x;
        let start_y = (rect.y - self.pan.y) / grid * grid + self.pan.y;

        let mut x = start_x;
        while x < rect.x + rect.w {
            ctx.renderer
                .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                    color: [0.2, 0.2, 0.2, 0.5],
                    rect: [x, rect.y, 1.0, rect.h],
                });
            x += grid;
        }

        let mut y = start_y;
        while y < rect.y + rect.h {
            ctx.renderer
                .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                    color: [0.2, 0.2, 0.2, 0.5],
                    rect: [rect.x, y, rect.w, 1.0],
                });
            y += grid;
        }
    }

    fn draw_connection(&self, ctx: &DrawContext, rect: Rect, conn: &NodeConnection) {
        if let (Some(from_node), Some(to_node)) = (
            self.nodes.get(&conn.from_node),
            self.nodes.get(&conn.to_node),
        ) {
            let from_port = from_node.outputs.iter().find(|p| p.id == conn.from_port);
            let to_port = to_node.inputs.iter().find(|p| p.id == conn.to_port);

            if let (Some(fp), Some(tp)) = (from_port, to_port) {
                let from_pos = self.port_position(&from_node, fp, true);
                let to_pos = self.port_position(&to_node, tp, false);

                let screen_from = self.graph_to_screen(from_pos);
                let screen_to = self.graph_to_screen(to_pos);

                // Bezier curve
                let cp1 = Point2::new(screen_from.x + 50.0, screen_from.y);
                let cp2 = Point2::new(screen_to.x - 50.0, screen_to.y);

                // Draw as line segments (simplified)
                ctx.renderer
                    .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                        color: [0.5, 0.5, 0.5, 0.8],
                        rect: [
                            screen_from.x,
                            screen_from.y,
                            screen_to.x - screen_from.x,
                            2.0,
                        ],
                    });
            }
        }
    }

    fn draw_drag_connection(&self, ctx: &DrawContext, rect: Rect, drag: &ConnectionDrag) {
        if let Some(from_node) = self.nodes.get(&drag.from_node) {
            let from_port = from_node
                .outputs
                .iter()
                .find(|p| p.id == drag.from_port)
                .or_else(|| from_node.inputs.iter().find(|p| p.id == drag.from_port));

            if let Some(fp) = from_port {
                let from_pos = self.port_position(&from_node, fp, !drag.is_input);
                let screen_from = self.graph_to_screen(from_pos);
                let screen_to = drag.current_pos;

                ctx.renderer
                    .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                        color: ctx.theme.accent,
                        rect: [
                            screen_from.x,
                            screen_from.y,
                            screen_to.x - screen_from.x,
                            2.0,
                        ],
                    });
            }
        }
    }

    fn port_position(&self, node: &Node, port: &NodePort, is_output: bool) -> Point2 {
        let node_screen = self.graph_to_screen(node.position);
        let port_index = if is_output {
            node.outputs
                .iter()
                .position(|p| p.id == port.id)
                .unwrap_or(0)
        } else {
            node.inputs
                .iter()
                .position(|p| p.id == port.id)
                .unwrap_or(0)
        };
        let port_count = if is_output {
            node.outputs.len()
        } else {
            node.inputs.len()
        };
        let spacing = node.size.1 / (port_count + 1) as f32;

        if is_output {
            Point2::new(
                node_screen.x + node.size.0 * self.zoom,
                node_screen.y + (port_index + 1) as f32 * spacing * self.zoom,
            )
        } else {
            Point2::new(
                node_screen.x,
                node_screen.y + (port_index + 1) as f32 * spacing * self.zoom,
            )
        }
    }

    fn draw_node(&self, ctx: &DrawContext, rect: Rect, node: &Node) {
        let screen_pos = self.graph_to_screen(node.position);
        let w = node.size.0 * self.zoom;
        let h = node.size.1 * self.zoom;
        let node_rect = Rect::new(screen_pos.x, screen_pos.y, w, h);

        // Node body
        let bg = if node.enabled {
            node.color
        } else {
            [
                node.color[0] * 0.5,
                node.color[1] * 0.5,
                node.color[2] * 0.5,
                0.5,
            ]
        };
        ctx.renderer
            .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color: bg,
                rect: [node_rect.x, node_rect.y, node_rect.w, node_rect.h],
            });

        // Selection border
        if node.selected {
            ctx.renderer
                .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                    color: ctx.theme.accent,
                    rect: [node_rect.x, node_rect.y, node_rect.w, 2.0],
                });
            ctx.renderer
                .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                    color: ctx.theme.accent,
                    rect: [
                        node_rect.x,
                        node_rect.y + node_rect.h - 2.0,
                        node_rect.w,
                        2.0,
                    ],
                });
            ctx.renderer
                .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                    color: ctx.theme.accent,
                    rect: [node_rect.x, node_rect.y, 2.0, node_rect.h],
                });
            ctx.renderer
                .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                    color: ctx.theme.accent,
                    rect: [
                        node_rect.x + node_rect.w - 2.0,
                        node_rect.y,
                        2.0,
                        node_rect.h,
                    ],
                });
        }

        // Title bar
        ctx.renderer
            .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color: [0.0, 0.0, 0.0, 0.8],
                rect: [node_rect.x, node_rect.y, node_rect.w, 24.0],
            });
        ctx.text_renderer.draw_text(
            &node.name,
            Point2::new(node_rect.x + 8.0, node_rect.y + 16.0),
            11.0,
            ctx.theme.text_primary,
        );

        // Enable/disable button
        let btn_color = if node.enabled {
            ctx.theme.success
        } else {
            ctx.theme.text_disabled
        };
        ctx.renderer
            .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color: btn_color,
                rect: [
                    node_rect.x + node_rect.w - 24.0,
                    node_rect.y + 2.0,
                    20.0,
                    20.0,
                ],
            });

        // Input ports
        for (i, port) in node.inputs.iter().enumerate() {
            let port_y = node_rect.y + 24.0 + (i + 1) as f32 * h / (node.inputs.len() + 1) as f32;
            let port_color = if port.connected.is_some() {
                ctx.theme.accent
            } else {
                ctx.theme.text_secondary
            };
            ctx.renderer
                .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                    color: port_color,
                    rect: [node_rect.x - 6.0, port_y - 6.0, 12.0, 12.0],
                });
            ctx.text_renderer.draw_text(
                &port.name,
                Point2::new(node_rect.x + 8.0, port_y + 4.0),
                9.0,
                ctx.theme.text_secondary,
            );
        }

        // Output ports
        for (i, port) in node.outputs.iter().enumerate() {
            let port_y = node_rect.y + 24.0 + (i + 1) as f32 * h / (node.outputs.len() + 1) as f32;
            let port_color = if port.connected.is_some() {
                ctx.theme.accent
            } else {
                ctx.theme.text_secondary
            };
            ctx.renderer
                .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                    color: port_color,
                    rect: [node_rect.x + w - 6.0, port_y - 6.0, 12.0, 12.0],
                });
            // Port name on right
            let name_w = port.name.len() as f32 * 5.0;
            ctx.text_renderer.draw_text(
                &port.name,
                Point2::new(node_rect.x + w - name_w - 8.0, port_y + 4.0),
                9.0,
                ctx.theme.text_secondary,
            );
        }
    }
}

impl Widget for NodeGraph {
    fn measure(&self, ctx: &LayoutContext, _: Constraints) -> (f32, f32) {
        (f32::INFINITY, f32::INFINITY)
    }

    fn layout(&mut self, _ctx: &LayoutContext, rect: Rect) {
        self.rect = rect;
    }

    fn draw(&self, ctx: &DrawContext, rect: Rect) {
        // Background
        ctx.renderer
            .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color: ctx.theme.bg_primary,
                rect: [rect.x, rect.y, rect.w, rect.h],
            });

        // Grid
        if self.show_grid {
            self.draw_grid(ctx, rect);
        }

        // Connections
        for conn in &self.connections {
            self.draw_connection(ctx, rect, conn);
        }

        // Connection being dragged
        if let Some(drag) = &self.connection_drag {
            self.draw_drag_connection(ctx, rect, drag);
        }

        // Nodes
        for node in self.nodes.values() {
            self.draw_node(ctx, rect, node);
        }
    }
    fn handle_input(&mut self, ctx: &InputContext, event: InputEvent) -> bool {
        match event {
            InputEvent::MouseDown(pos, btn) => {
                if pos.x < self.rect.x
                    || pos.x > self.rect.x + self.rect.w
                    || pos.y < self.rect.y
                    || pos.y > self.rect.y + self.rect.h
                {
                    return false;
                }

                let graph_pos = self.screen_to_graph(pos);

                match btn {
                    MouseButton::Left => {
                        // Check port clicks first
                        for node in self.nodes.values() {
                            let screen_pos = self.graph_to_screen(node.position);
                            let w = node.size.0 * self.zoom;
                            let h = node.size.1 * self.zoom;

                            // Input ports
                            for (i, port) in node.inputs.iter().enumerate() {
                                let port_y = screen_pos.y
                                    + 24.0
                                    + (i + 1) as f32 * h / (node.inputs.len() + 1) as f32;
                                let port_rect =
                                    Rect::new(screen_pos.x - 10.0, port_y - 10.0, 20.0, 20.0);
                                if pos.x >= port_rect.x
                                    && pos.x < port_rect.x + port_rect.w
                                    && pos.y >= port_rect.y
                                    && pos.y < port_rect.y + port_rect.h
                                {
                                    // Start connection drag from this input
                                    self.connection_drag = Some(ConnectionDrag {
                                        from_node: node.id,
                                        from_port: port.id.clone(),
                                        start_pos: pos,
                                        current_pos: pos,
                                        is_input: true,
                                    });
                                    return true;
                                }
                            }

                            // Output ports
                            for (i, port) in node.outputs.iter().enumerate() {
                                let port_y = screen_pos.y
                                    + 24.0
                                    + (i + 1) as f32 * h / (node.outputs.len() + 1) as f32;
                                let port_rect =
                                    Rect::new(screen_pos.x + w - 10.0, port_y - 10.0, 20.0, 20.0);
                                if pos.x >= port_rect.x
                                    && pos.x < port_rect.x + port_rect.w
                                    && pos.y >= port_rect.y
                                    && pos.y < port_rect.y + port_rect.h
                                {
                                    // Start connection drag from this output
                                    self.connection_drag = Some(ConnectionDrag {
                                        from_node: node.id,
                                        from_port: port.id.clone(),
                                        start_pos: pos,
                                        current_pos: pos,
                                        is_input: false,
                                    });
                                    return true;
                                }
                            }
                        }

                        // Check node click. Collect the hit first so the immutable borrow of
                        // `self.nodes` ends before the selection state is mutated.
                        let hit = self
                            .nodes
                            .values()
                            .find(|node| {
                                let screen_pos = self.graph_to_screen(node.position);
                                let w = node.size.0 * self.zoom;
                                let h = node.size.1 * self.zoom;
                                let node_rect = Rect::new(screen_pos.x, screen_pos.y, w, h);

                                pos.x >= node_rect.x
                                    && pos.x < node_rect.x + node_rect.w
                                    && pos.y >= node_rect.y
                                    && pos.y < node_rect.y + node_rect.h
                            })
                            .map(|node| (node.id, node.position));

                        if let Some((node_id, node_position)) = hit {
                            if ctx.key_modifiers.shift {
                                self.select_node(node_id, true);
                            } else if ctx.key_modifiers.ctrl {
                                if self.selected_nodes.contains(&node_id) {
                                    self.selected_nodes.retain(|&id| id != node_id);
                                    // Would update node.selected = false
                                } else {
                                    self.select_node(node_id, true);
                                }
                            } else {
                                self.deselect_all();
                                self.select_node(node_id, false);
                            }

                            self.drag_state = Some(NodeDragState {
                                node_id,
                                start_pos: node_position,
                                start_mouse: pos,
                                multi_select: self.selected_nodes.len() > 1,
                            });
                            return true;
                        }

                        // Click on empty space - deselect
                        if !ctx.key_modifiers.shift && !ctx.key_modifiers.ctrl {
                            self.deselect_all();
                        }

                        // Start pan
                        self.drag_state = Some(NodeDragState {
                            node_id: GradeNodeId::nil(),
                            start_pos: self.pan,
                            start_mouse: pos,
                            multi_select: false,
                        });
                        true
                    }
                    MouseButton::Right => {
                        // Context menu
                        true
                    }
                    _ => false,
                }
            }
            InputEvent::MouseMove(pos) => {
                if let Some(drag) = &mut self.drag_state {
                    if drag.node_id.0 == 0 {
                        // Pan
                        let delta =
                            Point2::new(pos.x - drag.start_mouse.x, pos.y - drag.start_mouse.y);
                        self.pan =
                            Point2::new(drag.start_pos.x + delta.x, drag.start_pos.y + delta.y);
                    } else if drag.multi_select {
                        // Move multiple selected nodes
                        let delta = Point2::new(
                            (pos.x - drag.start_mouse.x) / self.zoom,
                            (pos.y - drag.start_mouse.y) / self.zoom,
                        );
                        for node_id in &self.selected_nodes {
                            if let Some(node) = self.nodes.get_mut(node_id) {
                                node.position = Point2::new(
                                    drag.start_pos.x + delta.x,
                                    drag.start_pos.y + delta.y,
                                );
                            }
                        }
                    } else if let Some(node) = self.nodes.get_mut(&drag.node_id) {
                        // Move single node
                        let delta = Point2::new(
                            (pos.x - drag.start_mouse.x) / self.zoom,
                            (pos.y - drag.start_mouse.y) / self.zoom,
                        );
                        node.position =
                            Point2::new(drag.start_pos.x + delta.x, drag.start_pos.y + delta.y);
                    }

                    if let Some(conn_drag) = &mut self.connection_drag {
                        conn_drag.current_pos = pos;
                    }
                    true
                } else {
                    false
                }
            }
            InputEvent::MouseUp(pos, btn) => {
                if btn == MouseButton::Left {
                    if let Some(conn_drag) = self.connection_drag.take() {
                        // Check if dropped on a port
                        let graph_pos = self.screen_to_graph(pos);
                        for node in self.nodes.values() {
                            let screen_pos = self.graph_to_screen(node.position);
                            let w = node.size.0 * self.zoom;
                            let h = node.size.1 * self.zoom;

                            // Check input ports
                            for (i, port) in node.inputs.iter().enumerate() {
                                let port_y = screen_pos.y
                                    + 24.0
                                    + (i + 1) as f32 * h / (node.inputs.len() + 1) as f32;
                                let port_rect =
                                    Rect::new(screen_pos.x - 10.0, port_y - 10.0, 20.0, 20.0);
                                if pos.x >= port_rect.x
                                    && pos.x < port_rect.x + port_rect.w
                                    && pos.y >= port_rect.y
                                    && pos.y < port_rect.y + port_rect.h
                                {
                                    if conn_drag.is_input {
                                        // Input to input - invalid
                                    } else {
                                        // Output to input - valid
                                        self.connect(
                                            conn_drag.from_node,
                                            conn_drag.from_port,
                                            node.id,
                                            port.id.clone(),
                                        );
                                    }
                                    return true;
                                }
                            }

                            // Check output ports
                            for (i, port) in node.outputs.iter().enumerate() {
                                let port_y = screen_pos.y
                                    + 24.0
                                    + (i + 1) as f32 * h / (node.outputs.len() + 1) as f32;
                                let port_rect =
                                    Rect::new(screen_pos.x + w - 10.0, port_y - 10.0, 20.0, 20.0);
                                if pos.x >= port_rect.x
                                    && pos.x < port_rect.x + port_rect.w
                                    && pos.y >= port_rect.y
                                    && pos.y < port_rect.y + port_rect.h
                                {
                                    if !conn_drag.is_input {
                                        // Output to output - invalid
                                    } else {
                                        // Input to output - valid
                                        self.connect(
                                            node.id,
                                            port.id.clone(),
                                            conn_drag.from_node,
                                            conn_drag.from_port,
                                        );
                                    }
                                    return true;
                                }
                            }
                        }
                    }

                    self.drag_state = None;
                    true
                } else {
                    false
                }
            }
            InputEvent::Scroll(dx, dy) => {
                if dy != 0.0 {
                    let zoom_factor = 1.0 + dy * 0.1;
                    self.zoom = (self.zoom * zoom_factor).clamp(0.1, 5.0);
                    true
                } else {
                    false
                }
            }
            InputEvent::KeyDown(key) => {
                match key {
                    Key::Delete => {
                        for node_id in self.selected_nodes.clone() {
                            self.remove_node(node_id);
                        }
                        true
                    }
                    Key::A => {
                        if ctx.key_modifiers.ctrl {
                            // Select all
                            self.selected_nodes = self.nodes.keys().copied().collect();
                            for node in self.nodes.values_mut() {
                                node.selected = true;
                            }
                            true
                        } else {
                            false
                        }
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[derive(Clone)]
pub struct NodeGraphWidget {
    graph: NodeGraph,
    toolbar: NodeGraphToolbar,
}

impl NodeGraphWidget {
    pub fn new() -> Self {
        Self {
            graph: NodeGraph::new(),
            toolbar: NodeGraphToolbar::new(),
        }
    }

    pub fn set_timeline(&mut self, _timeline: Arc<RwLock<crate::timeline::clip::Timeline>>) {
        // Would sync with timeline
    }
}

impl Widget for NodeGraphWidget {
    fn measure(&self, ctx: &LayoutContext, _: Constraints) -> (f32, f32) {
        (f32::INFINITY, f32::INFINITY)
    }
    fn layout(&mut self, _ctx: &LayoutContext, rect: Rect) {
        self.graph.rect = rect;
        self.toolbar.rect = Rect::new(rect.x, rect.y, rect.w, 30.0);
    }
    fn draw(&self, ctx: &DrawContext, rect: Rect) {
        self.toolbar
            .draw(ctx, Rect::new(rect.x, rect.y, rect.w, 30.0));
        self.graph
            .draw(ctx, Rect::new(rect.x, rect.y + 30.0, rect.w, rect.h - 30.0));
    }
    fn handle_input(&mut self, ctx: &InputContext, event: InputEvent) -> bool {
        if self.toolbar.handle_input(ctx, event.clone()) {
            return true;
        }
        self.graph.handle_input(ctx, event)
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}

#[derive(Clone)]
struct NodeGraphToolbar {
    rect: Rect,
}

impl NodeGraphToolbar {
    fn new() -> Self {
        Self {
            rect: Rect::new(0.0, 0.0, 0.0, 30.0),
        }
    }
}

impl Widget for NodeGraphToolbar {
    fn measure(&self, ctx: &LayoutContext, _: Constraints) -> (f32, f32) {
        (f32::INFINITY, 30.0)
    }
    fn layout(&mut self, _ctx: &LayoutContext, rect: Rect) {
        self.rect = rect;
    }
    fn draw(&self, ctx: &DrawContext, rect: Rect) {
        ctx.renderer
            .submit(crate::gpu::renderer::RenderCommand::DrawQuad {
                color: ctx.theme.bg_secondary,
                rect: [rect.x, rect.y, rect.w, rect.h],
            });
        ctx.text_renderer.draw_text(
            "Node Graph",
            Point2::new(rect.x + 10.0, rect.y + 20.0),
            ctx.theme.font_size_sm,
            ctx.theme.text_primary,
        );
    }
    fn handle_input(&mut self, _ctx: &InputContext, _event: InputEvent) -> bool {
        false
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
}
