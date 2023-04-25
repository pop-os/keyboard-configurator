use cosmic::{
    iced::{Length, Point, Rectangle, Vector},
    iced_native::{
        event, layout, mouse, overlay, renderer,
        widget::{
            operation::{self, Operation},
            tree::{self, Tree},
        },
        Clipboard, Event, Shell, Size, Widget,
    },
};

pub struct FixedWidget<'a, Message> {
    children: Vec<(cosmic::Element<'a, Message>, Rectangle)>,
}

impl<'a, Message> FixedWidget<'a, Message> {
    pub fn new(children: Vec<(cosmic::Element<'a, Message>, Rectangle)>) -> Self {
        Self { children }
    }
}

impl<'a, Message> Widget<Message, cosmic::Renderer> for FixedWidget<'a, Message> {
    fn children(&self) -> Vec<Tree> {
        self.children
            .iter()
            .map(|(child, _)| Tree::new(child))
            .collect()
    }

    fn width(&self) -> Length {
        Length::Shrink
    }

    fn height(&self) -> Length {
        Length::Shrink
    }

    fn layout(&self, renderer: &cosmic::Renderer, _limits: &layout::Limits) -> layout::Node {
        // TODO handle limits in some way?
        let mut total_size = Size::new(0.0f32, 0.0);
        let children = self
            .children
            .iter()
            .map(|(child, rectangle)| {
                total_size.width = total_size.width.max(rectangle.x + rectangle.width);
                total_size.height = total_size.height.max(rectangle.y + rectangle.height);

                let size = rectangle.size();
                let limits = layout::Limits::new(size, size);
                let vector = Vector::new(rectangle.x, -rectangle.y);
                child
                    .as_widget()
                    .layout(renderer, &limits)
                    .translate(vector)
            })
            .collect();
        layout::Node::with_children(total_size, children)
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
            .map(|(child, _)| child)
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

impl<'a, Message> From<FixedWidget<'a, Message>> for cosmic::Element<'a, Message>
where
    Message: 'a,
{
    fn from(row: FixedWidget<'a, Message>) -> Self {
        Self::new(row)
    }
}
