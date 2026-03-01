use super::{App, AppState};
use crate::config::save_last_context;
use crate::context::{CloudContext, ContextSelectorView};
use crate::registry::ServiceId;
use crate::service::{Service, ServiceSelectorView};

impl App {
    pub(super) fn start_service(&mut self, context: &CloudContext, service_id: &ServiceId) {
        self.active_context = Some(context.clone());
        self.status_bar.set_active_context(context.clone());
        if let Some(provider) = self.registry.get(service_id) {
            let service = provider.create_service(context);
            self.go_to_active_service(service);
        }
    }

    pub(super) fn go_to_filtered_context_selection(&mut self, contexts: Vec<CloudContext>) {
        self.state =
            AppState::SelectingContext(ContextSelectorView::new(contexts));
    }

    /// Transition to context selection.
    pub(super) fn go_to_context_selection(&mut self) {
        self.active_context = None;
        self.status_bar.clear_context();
        let contexts = self.context_manager.get_all();
        self.state =
            AppState::SelectingContext(ContextSelectorView::new(contexts));
    }

    /// Transition to service selection.
    pub(super) fn go_to_service_selection(&mut self, context: &CloudContext) {
        self.active_context = Some(context.clone());
        self.status_bar.set_active_context(context.clone());
        self.state = AppState::SelectingService(ServiceSelectorView::new(
            &self.registry,
            context,
        ));
    }

    /// Transition to active service.
    pub(super) fn go_to_active_service(&mut self, mut service: Box<dyn Service>) {
        // Save last context for -s flag
        if let Some(ctx) = &self.active_context {
            let _ = save_last_context(ctx.name());
        }

        // Initialize the service (queues startup message)
        service.init();
        self.state = AppState::ActiveService(service);

        // Immediately process the startup message
        if let AppState::ActiveService(service) = &mut self.state {
            let result = service.update();
            self.process_update_result(result);
        }
    }

    /// Handle going back one state.
    pub(super) fn go_back(&mut self) {
        match &mut self.state {
            AppState::SelectingContext(_) => {}
            AppState::SelectingService(_) => {
                self.go_to_context_selection();
            }
            AppState::ActiveService(service) => {
                service.destroy();
                if let Some(ref ctx) = self.active_context.clone() {
                    self.go_to_service_selection(ctx);
                } else {
                    self.go_to_context_selection();
                }
            }
        }
    }
}
