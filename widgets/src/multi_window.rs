
use {
    crate::{
        widget::*,
        makepad_derive_widget::*,
        desktop_window::*,
        makepad_draw::*,
    }
};

live_design!{
    import makepad_draw::shader::std::*;
    import makepad_widgets::theme::*;
    
    MultiWindow = {{MultiWindow}} {
    }
}

#[derive(Live)]
pub struct MultiWindow {
    #[rust] draw_state: DrawStateWrap<DrawState>,
    #[rust] windows: ComponentMap<LiveId, DesktopWindow>,
}

impl LiveHook for MultiWindow {
    fn before_live_design(cx:&mut Cx){
        register_widget!(cx,MultiWindow)
    }
    
    fn apply_value_instance(&mut self, cx: &mut Cx, from: ApplyFrom, index: usize, nodes: &[LiveNode]) -> usize {
        let id = nodes[index].id;
        match from {
            ApplyFrom::NewFromDoc {..} | ApplyFrom::UpdateFromDoc {..} => {
                if nodes[index].origin.has_prop_type(LivePropType::Instance) {
                    return self.windows.get_or_insert(cx, id, | cx | {DesktopWindow::new(cx)})
                        .apply(cx, from, index, nodes);
                }
                else {
                    cx.apply_error_no_matching_field(live_error_origin!(), index, nodes);
                }
            }
            _ => ()
        }
        nodes.skip_node(index)
    }
}

#[derive(Clone)]
enum DrawState {
    Window(usize),
}

impl Widget for MultiWindow {
    fn redraw(&mut self, cx: &mut Cx) {
        for window in self.windows.values_mut() {
            window.redraw(cx);
        }
    }
    
    fn find_widgets(&mut self, path: &[LiveId], cached: WidgetCache, results:&mut WidgetSet){
        for window in self.windows.values_mut() {
            window.find_widgets(path, cached, results);
        }
    }
    
    fn handle_widget_event_with(&mut self, cx: &mut Cx, event: &Event, dispatch_action: &mut dyn FnMut(&mut Cx, WidgetActionItem)) {
        for window in self.windows.values_mut() {
            window.handle_widget_event_with(cx, event, dispatch_action);
        }
    }
    
    fn get_walk(&self) -> Walk {Walk::default()}
    
    fn draw_walk_widget(&mut self, cx: &mut Cx2d, _walk: Walk) -> WidgetDraw {
        self.draw_state.begin(cx, DrawState::Window(0));
        while let Some(DrawState::Window(step)) = self.draw_state.get() {
            if let Some(window) = self.windows.values_mut().nth(step){
                window.draw_widget_hook(cx)?; 
                self.draw_state.set(DrawState::Window(step+1));
            }
            else{
                self.draw_state.end();
            }
        }
        WidgetDraw::done()
    }
}

#[derive(Clone, Default, PartialEq, WidgetRef)]
pub struct MultiWindowRef(WidgetRef);