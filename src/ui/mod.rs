//! Custom lightweight UI framework for DaVinci Clone
//! No egui dependency - custom immediate mode on wgpu for minimum size

pub mod widgets;
pub mod layout;
pub mod theme;
pub mod input;
pub mod pages;
pub mod timeline_widget;
pub mod viewer;
pub mod scopes_widget;
pub mod mixer;
pub mod inspector;
pub mod media_pool_widget;
pub mod node_graph;

// Explicit re-exports to keep names unambiguous across submodules.

pub use theme::{Theme, Shadow};
pub use layout::{
    Widget, LayoutContext, DrawContext, InputContext, TextRenderer, Constraints, LayoutResult,
    Direction, Align, Flex, FlexChild, Grid, InputEvent, MouseButton, Key, KeyModifiers,
};
pub use widgets::{
    Button, ButtonVariant, ButtonSize, Label, LabelSize, TextAlign, Input, InputType, Slider,
    Checkbox, Separator, SeparatorOrientation, Tooltip, TooltipPosition, Panel, ScrollArea,
};
pub use input::{InputState, DragState, ClickDetector};
pub use pages::{
    CutPage, EditPage, EditPanel, LayoutMode, ColorPage, FusionPage, FairlightPage, DeliverPage,
};