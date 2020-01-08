#![recursion_limit="1024"]
extern crate stdweb;
use stdweb::unstable::TryInto;
use stdweb::{js};
use stdweb::traits::*;
use stdweb::web::html_element::InputElement;
use stdweb::web::{HtmlElement, document, WebSocket, Element, MutationObserver};
use stdweb::web::event::{KeyPressEvent, SocketOpenEvent, SocketCloseEvent, SocketErrorEvent, SocketMessageEvent, ClickEvent};

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

fn location_query(){
    let me = WebSocket::new("ws://0.0.0.0:8844/websockets/gmaps_queries").unwrap();
    let ws = WebSocket::new("ws://0.0.0.0:8844/websockets/va_facilities").unwrap();
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
            let result = js! {
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
                            console.log(va_facs[j].attributes);
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
                        }if(va_facs[j].attributes.name === e.target._popup._content && va_facs[j].attributes.phone.main != null){
                            var phone_p = document.createElement("p");
                            var txt = "phone number :"+va_facs[j].attributes.phone.main;
                            phone_p.innerHTML = txt;
                            document.getElementById("details").append(phone_p);
                        }if(va_facs[j].attributes.name === e.target._popup._content && va_facs[j].attributes.hours != null){
                            console.log(va_facs[j].attributes);
                            var hours_tag = document.createElement("p");
                            hours_tag.innerHTML = "Hours For Facility Below";
                            var hours_p = document.createElement("p");
                            var hours_ul = document.createElement("ul");
                            for(var key in va_facs[j].attributes.hours){
                                var new_li = document.createElement("li");
                                var txt = key + " : " + va_facs[j].attributes.hours[key];
                                new_li.innerHTML = txt;
                                hours_ul.append(new_li);
                            }
                            hours_p.append(hours_ul);
                            document.getElementById("details").append(hours_tag);
                            document.getElementById("details").append(hours_p);
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
                            let socket = new WebSocket("ws://0.0.0.0:8844/websockets/gmaps_queries");
                            socket.onopen = function(event){
                                socket.send("directions,latlng="+dest_loc);
                            };
                            socket.onmessage = function(event){
                                console.log(event.data);
                                var dir_doc = document.getElementById("detail_directions");
                                var directions = event.data.split("%");
                                var text_directions = directions[0].split("/n");
                                var polyline = directions[1];
                                console.log(polyline);
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
                                map.removeLayer();
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
    stdweb::initialize();
    location_query();
}