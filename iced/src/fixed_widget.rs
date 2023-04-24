use cosmic::{
    iced::{Length, Point, Rectangle, Vector},
    iced_native::{
    event, layout, mouse, overlay, renderer,
    widget::{
        operation::{self, Operation},
        tree::{self, Tree},
    },
    Clipboard, Event, Shell, Widget,
}};

struct FixedWidget<'a, Message> {
    children: Vec<cosmic::Element<'a, Message>>,
}

impl<'a, Message> Widget<Message, cosmic::Renderer> for FixedWidget<'a, Message> {
    fn children(&self) -> Vec<Tree> {
        self.children.iter().map(Tree::new).collect()
    }

    fn width(&self) -> Length {
        todo!()
    }

    fn height(&self) -> Length {
        todo!()
    }

    fn layout(&self, renderer: &cosmic::Renderer, limits: &layout::Limits) -> layout::Node {
        todo!()
    }

    // TODO operate, on_event, mouse_interaction

    fn draw(
        &self,
        tree: &Tree,
        renderer: &mut cosmic::Renderer,
        theme: &cosmic::Theme,
        style: &renderer::Style,
        layout: layout::Layout<'_>,
        cursor_position: Point,
        viewport: &Rectangle,
    ) {
        for ((child, state), layout) in self
            .children
            .iter()
            .zip(&tree.children)
            .zip(layout.children())
        {
            child.as_widget().draw(
                state,
                renderer,
                theme,
                style,
                layout,
                cursor_position,
                viewport,
            );
        }
    }
}
