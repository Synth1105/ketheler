//! Agent macro for minimal state holders.
//!
//! Enabled with the `agent` feature.

/// Generates a simple Agent server for a given state type.
///
/// The state type must implement:
/// `Default + Clone + Debug + Send + 'static`.
///
/// # Example
/// ```rust
/// ketheler::agent!(u64);
///
/// let handle = Agent::start_link();
/// let value = Agent::get(&handle, |v| v);
/// let updated = Agent::get_and_update(&handle, |v| v + 1);
/// let set = Agent::update(&handle, 42);
/// ```
#[cfg(feature = "agent")]
#[macro_export]
macro_rules! agent {
    ($state_ty:ty) => {
        use $crate::server::{self, Response, ResponseKind, Server, ServerHandle};
        use std::fmt::Debug;

        pub struct Agent;

        pub enum Call {
            Get,
        }

        pub enum Cast {
            Update($state_ty),
        }

        impl Server for Agent
        where
            $state_ty: Clone + Default + Send + 'static,
        {
            type State = $state_ty;
            type Call = Call;
            type Cast = Cast;
            type Info = ();
            type Reply = $state_ty;

            fn init() -> Self::State {
                <$state_ty as Default>::default()
            }

            fn handle_call(
                call: Self::Call,
                _from: server::CallRef<Self::Reply>,
                state: Self::State,
            ) -> Response<Self::Reply, Self::State> {
                match call {
                    Call::Get => Response::Reply(state.clone(), state, Some(ResponseKind::Call)),
                }
            }

            fn handle_cast(
                cast: Self::Cast,
                _state: Self::State,
            ) -> Response<Self::Reply, Self::State> {
                match cast {
                    Cast::Update(val) => Response::NoReply(val, Some(ResponseKind::Cast)),
                }
            }
        }

        impl Agent {
            pub fn start_link() -> ServerHandle<Agent>
            where
                $state_ty: Debug,
            {
                server::start_link::<Agent>()
            }

            pub fn get<R, F>(handle: &ServerHandle<Agent>, func: F) -> R
            where
                F: FnOnce($state_ty) -> R,
                $state_ty: Debug,
            {
                let state = handle
                    .call(Call::Get)
                    .expect("agent call failed");
                func(state)
            }

            pub fn get_and_update<F>(handle: &ServerHandle<Agent>, func: F) -> $state_ty
            where
                F: FnOnce($state_ty) -> $state_ty,
                $state_ty: Debug + Clone,
            {
                let state = handle
                    .call(Call::Get)
                    .expect("agent call failed");
                let updated = func(state);
                handle
                    .cast(Cast::Update(updated.clone()))
                    .expect("agent cast failed");
                updated
            }

            pub fn update(handle: &ServerHandle<Agent>, val: $state_ty) -> $state_ty
            where
                $state_ty: Debug + Clone,
            {
                handle
                    .cast(Cast::Update(val.clone()))
                    .expect("agent cast failed");
                val
            }
        }
    };
}

#[cfg(all(test, feature = "agent"))]
mod tests {
    mod u64_agent {
        crate::agent!(u64);

        #[test]
        fn get_update_flow() {
            let handle = Agent::start_link();
            let value = Agent::get(&handle, |v| v);
            assert_eq!(value, 0);

            let updated = Agent::get_and_update(&handle, |v| v + 1);
            assert_eq!(updated, 1);

            let value = Agent::get(&handle, |v| v);
            assert_eq!(value, 1);

            let set = Agent::update(&handle, 10);
            assert_eq!(set, 10);

            let value = Agent::get(&handle, |v| v);
            assert_eq!(value, 10);
        }
    }
}
