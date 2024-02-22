use gloo_net::http::Request;
use gloo_net::websocket;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_router::{BrowserRouter, Routable, Switch};

#[derive(Clone, Routable, PartialEq)]
enum Route {
    #[at("/")]
    Home,
    #[at("/hello-server")]
    HelloServer,
}

fn switch(routes: Route) -> Html {
    match routes {
        Route::Home => html! {
        <div>
            <h1>{"Hello Frontend"}</h1>
        <a href = "/hello-server">{"Go to server"}</a>
        </div>
        },
        Route::HelloServer => html! {<HelloServer/>},
    }
}

#[function_component]
fn App() -> Html {
    html! {
    <BrowserRouter>
        <Switch<Route>render={switch}/>
        </BrowserRouter>
    }
}

#[function_component(HelloServer)]
fn hello_server() -> Html {
    let data = use_state(|| None);

    {
        let data = data.clone();
        use_effect(move || {
            if data.is_none() {
                spawn_local(async move {
                    let resp = Request::get("/api/hello").send().await.unwrap();
                    let result = {
                        if !resp.ok() {
                            Err(format!(
                                "Error fetching data {} ({})",
                                resp.status(),
                                resp.status_text()
                            ))
                        } else {
                            resp.text().await.map_err(|err| err.to_string())
                        }
                    };
                    data.set(Some(result));
                })
            }
            || {}
        });
    }
    match data.as_ref() {
        None => {
            html! {
            <div>{"No server response"}</div>
            }
        }
        Some(Ok(data)) => {
            html! {
            <div>{"Got a server response: "}{data}</div>
            }
        }
        Some(Err(err)) => {
            html! {
            <div>{"Error requesting data from server: "}{err}</div>
            }
        }
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
