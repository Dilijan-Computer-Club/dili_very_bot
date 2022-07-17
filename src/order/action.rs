
use crate::order::{ActionKind, OrderId};

/// ActionKind for specific order
#[derive(Clone, Debug)]
pub struct Action {
    pub order_id: OrderId,
    pub kind: ActionKind,
}

impl Action {
    const BTN_DATA_PREFIX: &'static str = "oa";
    pub fn human_name(&self) -> &'static str {
        self.kind.human_name()
    }

    /// Serializes it in a way that can be parsed by `try_parse`
    pub fn kbd_button_data(&self) -> String {
    format!("{} {} {}",
            Action::BTN_DATA_PREFIX,
            self.kind.id(),
            self.order_id.0)
    }

    /// If `data` can be parsed as SpecificAtion it returns it, otherwise None
    ///
    /// `actor` is passed in separately because passing it as data is
    /// probably not safe, and it can be found in the callback
    pub fn try_parse(data: &str) -> Option<Action> {
        let mut args = data.split(' ');

        let magic = args.next()?;
        if magic != Self::BTN_DATA_PREFIX { return None }

        let kind = args.next()?;
        let order_id = args.next()?;
        let order_id = OrderId(order_id.parse().ok()?);

        // Too many arguments
        if args.next().is_some() { return None }

        let kind = ActionKind::maybe_from_id(kind)?;
        Some(Action { kind, order_id })
    }
}

