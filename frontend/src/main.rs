use futures_util::{SinkExt, StreamExt};
use gloo_console::log;
use gloo_net::http::Request;
use gloo_net::websocket::{futures::WebSocket, Message};
use gloo_timers::future::TimeoutFuture;
use wasm_bindgen_futures::spawn_local;
use yew::prelude::*;
use yew_router::{BrowserRouter, Routable, Switch};

#[derive(Clone, Routable, PartialEq)]
enum Route {
    #[at("/")]
    Home,
}

fn switch(routes: Route) -> Html {
    match routes {
        Route::Home => html! {<Main/>},
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

#[function_component(Main)]
fn main() -> Html {
    use_effect_with((), move |_| {
        match WebSocket::open("ws://127.0.0.1:8080/ws/ws") {
            Ok(ws) => {
                let (mut write, mut read) = ws.split();
                spawn_local(async move {
                    while let Some(m) = read.next().await {
                        log!(format!("Got message: {:?}", m));
                        write.send(Message::Text("Hi".into())).await.unwrap();
                    }
                    log!("Bye bye socket");
                })
            }
            Err(e) => {
                log!(format!("Couldn't open websocket: {:?}", e));
            }
        }
    });
    html! {"Hello"}
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
