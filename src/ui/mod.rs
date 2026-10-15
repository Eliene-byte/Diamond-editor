//! Custom lightweight UI framework for DaVinci Clone
//! No egui dependency - custom immediate mode on wgpu for minimum size

pub mod input;
pub mod inspector;
pub mod layout;
pub mod media_pool_widget;
pub mod mixer;
pub mod node_graph;
pub mod pages;
pub mod scopes_widget;
pub mod theme;
pub mod timeline_widget;
pub mod viewer;
pub mod widgets;

// Explicit re-exports to keep names unambiguous across submodules.

pub use input::{ClickDetector, DragState, InputState};
pub use layout::{
    Align, Constraints, Direction, DrawContext, Flex, FlexChild, Grid, InputContext, InputEvent,
    Key, KeyModifiers, LayoutContext, LayoutResult, MouseButton, TextRenderer, Widget,
};
pub use pages::{
    ColorPage, CutPage, DeliverPage, EditPage, EditPanel, FairlightPage, FusionPage, LayoutMode,
};
pub use theme::{Shadow, Theme};
pub use widgets::{
    Button, ButtonSize, ButtonVariant, Checkbox, Input, InputType, Label, LabelSize, Panel,
    ScrollArea, Separator, SeparatorOrientation, Slider, TextAlign, Tooltip, TooltipPosition,
};
