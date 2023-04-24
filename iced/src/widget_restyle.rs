/*
use cosmic::iced_native::{
    event, layout, mouse, overlay, renderer,
    widget::{
        operation::{self, Operation},
        tree::{self, Tree},
    },
    Clipboard, Event, Shell, Widget,
};
use iced::{Length, Point, Quad, Rectangle, Vector};
use std::marker::PhantomData;

struct RendererAdapter<'a>(&cosmic::Renderer);

impl<'a> renderer::Renderer for RendererAdapter<'a> {
    type Theme = iced::Theme;

    delegate::delegate! {
        to self.0 {
            fn layout<'a, Message>(
                &mut self,
                element: &Element<'a, Message, Self>,
                limits: &layout::Limits,
            ) -> layout::Node;
            fn with_layer(&mut self, bounds: Rectangle, f: impl FnOnce(&mut Self));
            fn with_translation(
                &mut self,
                translation: Vector,
                f: impl FnOnce(&mut Self));
            fn clear(&mut self);
            fn fill_quad(&mut self, quad: Quad, background: impl Into<Background>);
        }
    }
}

struct WidgetRestyle<Message, T: Widget<Message, iced::Renderer>>(T, PhantomData<Message>);

impl<Message, T: Widget<Message, RendererAdapter>> Widget<Message, cosmic::Renderer>
    for WidgetRestyle<Message, T>
{
    // Hm, can't delegate with methods with renderer
    // Also need renderer adapter?
    delegate::delegate! {
        to self.0 {
            fn tag(&self) -> tree::Tag;
            fn state(&self) -> tree::State;
            fn children(&self) -> Vec<Tree>;
            fn diff(&self, tree: &mut Tree);
            fn width(&self) -> Length;
            fn height(&self) -> Length;
            fn operate(
                &self,
                tree: &mut Tree,
                layout: layout::Layout<'_>,
                operation: &mut dyn Operation<Message>);
        }
    }

    fn layout(
        &self,
        renderer: &cosmic::Renderer,
        limits: &layout::Limits,
    ) -> layout::Node {
        self.0.layout(RendererAdapter(renderer), limits)
    }

    /*
        fn layout(
            &self,
            renderer: &cosmic::Renderer,
            limits: &layout::Limits,
        ) -> layout::Node;
        fn on_event(
            &mut self,
            tree: &mut Tree,
            event: Event,
            layout: layout::Layout<'_>,
            cursor_position: Point,
            renderer: &cosmic::Renderer,
            clipboard: &mut dyn Clipboard,
            shell: &mut Shell<'_, Message>,
        ) -> event::Status;
        fn draw(
            &self,
            tree: &Tree,
            renderer: &mut cosmic::Renderer,
            theme: &cosmic::Theme,
            renderer_style: &renderer::Style,
            layout: layout::Layout<'_>,
            cursor_position: Point,
            _viewport: &Rectangle,
        );

        fn mouse_interaction(
            &self,
            _tree: &Tree,
            layout: layout::Layout<'_>,
            cursor_position: Point,
            _viewport: &Rectangle,
            _renderer: &cosmic::Renderer,
        ) -> mouse::Interaction;

        fn overlay<'b>(
            &'b self,
            tree: &'b mut Tree,
            layout: layout::Layout<'_>,
            renderer: &cosmic::Renderer,
        ) -> Option<overlay::Element<'b, Message, cosmic::Renderer>>;
    }
    }
    */
}
*/
