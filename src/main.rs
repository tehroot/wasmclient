#![recursion_limit="1024"]
extern crate stdweb;
extern crate serde_json;

use stdweb::unstable::TryInto;
use stdweb::{js};
use stdweb::traits::*;
use stdweb::web::html_element::InputElement;
use stdweb::web::{HtmlElement, document, WebSocket, Element, SocketReadyState};
use stdweb::web::event::{KeyPressEvent, SocketOpenEvent, SocketCloseEvent, SocketErrorEvent, SocketMessageEvent, ClickEvent, SocketMessageData};
use serde_json::{Result, Value};
use std::cell::{RefCell};
use std::rc::Rc;

//creates a macro in rust to perform some expression/ownership enclosing
macro_rules! enclose {
//macro magic
    (($( $x:ident ),*) $y:expr ) => {
        {
            //clone object
            $(let $x = $x.clone();)*
            $y
        }
    };
}

struct UserLoc {
    latitude: i64,
    longitude: i64
}

impl UserLoc {
    fn new() -> Self {
        UserLoc {
            latitude: 0,
            longitude: 0
        }
    }

    fn parse_coords(&mut self, input_coord: &str) -> Result<()>{
        self.latitude = js!{
            return JSON.parse(@{ input_coord }).lat;
        }.try_into().unwrap();
        self.longitude = js!{
            return JSON.parse(@{ input_coord }).lng;
        }.try_into().unwrap();

        js!{console.log(@{ self.latitude.to_string() });};
        Ok(())
        //self.longitude = v["lng"].as_i64().unwrap();
        //self.latitude = v["lat"].as_i64().unwrap();
    }

    fn set_lat(&mut self, latitude: i64){
        self.latitude = latitude;
    }

    fn set_lon(&mut self, longitude: i64){
        self.longitude = longitude
    }
}

fn facilities_query(ws: WebSocket, geocode: String, user_pos: Rc<RefCell<UserLoc>>) {
    //let ws = WebSocket::new("ws://localhost:8844/websockets/gmaps_queries").unwrap();
    ws.send_text(&geocode);
    ws.add_event_listener(enclose!((user_pos) move | event: SocketMessageEvent| {
        let str = &event.data().into_text().unwrap().clone();
        user_pos.borrow_mut().parse_coords(str);
    }));
}

fn location_query(){
    let me = WebSocket::new("ws://localhost:8844/websockets/data").unwrap();
    let ws = WebSocket::new("ws://localhost:8844/websockets/data").unwrap();
    let text_entry: InputElement = document().query_selector(".form-input input").unwrap().unwrap().try_into().unwrap();
    me.add_event_listener(move |event : SocketMessageEvent| {
        let fac_query: String = (&event.data().into_text().unwrap().to_owned()).parse().unwrap();
        let coords: String = js!{
            var fac_query = @{&fac_query};
            var lat = JSON.parse(fac_query).lat;
            var lng = JSON.parse(fac_query).lng;
            return '"'+lat +","+lng+'"';
        }.try_into().unwrap();
        js! {
            var container = L.DomUtil.get("map");
            var latlngs = JSON.parse(@{&event.data().into_text().unwrap()});
            if(container != null){
                container.parentNode.removeChild(container);
                var new_div = document.createElement("div");
                new_div.setAttribute("id", "map");
                new_div.className = "container-fluid";
                new_div.className += " mapcontainer";
                document.getElementById("mapcontainer").append(new_div);
                map = L.map("map").setView([latlngs.lat, latlngs.lng], 13);
                L.tileLayer("https://tile.openstreetmap.org/{z}/{x}/{y}.png").addTo(map);
            } else {
                map = L.map("map").setView([latlngs.lat, latlngs.lng], 13);
                L.tileLayer("https://tile.openstreetmap.org/{z}/{x}/{y}.png").addTo(map);
            }
        };
        let coords_owned: String = "facilities,latlng=".to_owned()+&coords.to_string();
        ws.send_text(&coords_owned).unwrap();
        ws.add_event_listener(move |event : SocketMessageEvent| {
            let fac_results: String = (&event.data().into_text().unwrap().to_owned()).parse().unwrap();
            js! {
                var va_facs = new Array();
                var splitStrings = @{fac_results}.trim().split("/n");
                for(var i=0; i<splitStrings.length; i++){
                    var fac = JSON.parse(splitStrings[i]);
                    va_facs.push(fac);
                    var marker = new L.marker([fac.attributes.lat, fac.attributes.long]).bindPopup(fac.attributes.name);
                    marker.addTo(map).on("click", onClick);
                }
                function onClick(e){
                    var dirdiv = document.getElementById("details");
                    var dirdiv_details = document.getElementById("detail_directions");
                    dirdiv.innerHTML = "";
                    dirdiv_details.innerHTML = "";
                    for(var j=0;j<va_facs.length;j++){
                        if(va_facs[j].attributes.name === e.target._popup._content){
                            var title_p = document.createElement("h1");
                            var name = va_facs[j].attributes.name;
                            title_p.id = "location-name";
                            title_p.innerHTML = name;
                            document.getElementById("details").append(title_p);
                            for(var key in va_facs[j].attributes){
                                if(key == "facility_type" || key == "classification"){
                                    var new_p = document.createElement("p");
                                    var txt = key + " : "+ va_facs[j].attributes[key];
                                    new_p.innerHTML = txt;
                                    document.getElementById("details").append(new_p);
                                }
                            }
                            var divRow = document.createElement("div");
                            divRow.className = "row";
                            dirdiv.append(divRow);
                        }
                    }
                    appendDirectionButton(dirdiv);
                }

                function appendDirectionButton(elem){
                    var button = document.createElement("button");
                    button.className = "btn-btn primary";
                    button.id = "get-directions";
                    button.innerHTML = "Get Directions";
                    button.onclick = getDirections;
                    elem.append(button);
                }

                function getDirections(){
                    var dest_name = document.getElementById("location-name").innerHTML;
                    var dest_loc;
                    for(var l=0;l<va_facs.length;l++){
                        if(va_facs[l].attributes.name === dest_name){
                            dest_loc = "["+map.getCenter().lat+";"+map.getCenter().lng+";"+va_facs[l].attributes.lat +";"+va_facs[l].attributes.long+"]";
                            let socket = new WebSocket("ws://localhost:8844/websockets/data");
                            socket.onopen = function(event){
                                socket.send("directions,latlng="+dest_loc);
                            };
                            socket.onmessage = function(event){
                                var dir_doc = document.getElementById("detail_directions");
                                var directions = event.data.split(";");
                                var text_directions = directions[0].split("/n");
                                var polyline = directions[1];
                                var latlng_arr = JSON.parse(directions[2]);
                                text_directions.pop();
                                for(var m=0;m<text_directions.length;m++){
                                    console.log(text_directions[m]);
                                    var new_direc_step = document.createElement("p");
                                    new_direc_step.innerHTML = text_directions[m];
                                    dir_doc.append(new_direc_step);
                                }
                                var polyline = L.polyline(latlng_arr, {color: "red", weight: 6}).addTo(map);
                                socket.close();
                            }
                        }
                    }
                }
            };
        });
    });
    text_entry.add_event_listener(enclose!((text_entry) move |event: KeyPressEvent| {
        if event.key() == "Enter" {
            event.prevent_default();
            let text: String = text_entry.raw_value();
            if text.is_empty() == false {
                text_entry.set_raw_value("");
                let owned_txt: String = "geocode,query=".to_owned()+&text.to_string();
                me.send_text(&owned_txt).unwrap();
            }
        }
    }));
}

fn main() {
    let ws = WebSocket::new("ws://localhost:8844/websockets/gmaps_queries").unwrap();
    //RefCell -- sharable mutable container, not thread safe
    let init_userloc = Rc::new(RefCell::new(UserLoc::new()));
    let text_entry: InputElement = document().query_selector(".form-input input").unwrap().unwrap().try_into().unwrap();
    text_entry.add_event_listener(enclose!((text_entry) move |event: KeyPressEvent| {
        if event.key() == "Enter" {
            event.prevent_default();
            let text: String = text_entry.raw_value();
            if text.is_empty() == false {
                text_entry.set_raw_value("");
                let owned_txt: String = "geocode,query=".to_owned()+&text.to_string();
                //clone into closure
                facilities_query(ws.clone(), owned_txt, init_userloc.clone());
            }
        }
    }));
    stdweb::initialize();
    stdweb::event_loop();
    //location_query();
}