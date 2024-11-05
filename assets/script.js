class WebSocketClient {

    webSocket;

    newPoint;

    constructor(newPoint) {
        this.newPoint = newPoint;
        this._generate();
    }

    _generate() {
        const url = new URL('/api/public/v1/map/ws', window.location.href);
        url.protocol = url.protocol.replace('http', 'ws');
        this.webSocket = new WebSocket(url.href);

        this.webSocket.onopen = (event) => {
            console.log("WebSocket opened");
        }

        this.webSocket.onclose = (event) => {
            console.log("WebSocket closed");
            this._reset();
        }

        this.webSocket.onerror = (event) => {
            console.log("WebSocket error");
            this._reset();
        }

        this.webSocket.onmessage = (event) => {
            console.log("WebSocket message");
            const data = JSON.parse(event.data);

            const band = data.band;
            const latitude = data.latitude;
            const longitude = data.longitude;
            console.log(`Latitude: ${latitude}, longitude: ${longitude}, band: ${band}`);

            this.newPoint(latitude, longitude, band);
        }
    }

    _reset() {
        this.webSocket.close();
        this.webSocket = null;

        setTimeout(() => {
            this._generate();
        }, 2000);
    }
}

class PointHandler {
    map;
    points = [];

    constructor(map) {
        this.map = map;
    }

    addPoint(point) {
        const [marker, geodesic] = point;
        marker.addTo(this.map);
        geodesic.addTo(this.map);

        this.points.push(point);

        while (this.points.length > 10) {
            const [marker, geodesic] = this.points.shift();
            this.map.removeLayer(marker);
            this.map.removeLayer(geodesic);
        }
    }
}

function initMap(divId) {
    let map = L.map(divId, {
        zoomControl: false
    });

    map.setView([0.0, 0.0], 2);

    L.tileLayer('https://tile.openstreetmap.org/{z}/{x}/{y}.png', {
        map: map,
        maxZoom: 19,
        attribution: '&copy; <a href="http://www.openstreetmap.org/copyright">OpenStreetMap</a>'
    }).addTo(map);

    return map;
}

async function retrieveHomePoint() {
    const response = await window.fetch('/api/public/v1/points/home')
    const response_body = await response.json();
    return new L.latLng(response_body.latitude, response_body.longitude);
}

function generateMarkerGeodesic(pointFrom, pointTo, geodesicColor) {
    const marker = L.marker(pointTo, {
        opacity: 1
    });

    const geodesic = L.geodesic([pointFrom, pointTo], {
        weight: 1,
        color: geodesicColor
    });

    return [marker, geodesic];
}

function computeColorByBand(band) {
    switch (band) {
        case '10':
            return '#1c65cd';
        case '15':
            return '#109e0e';
        case '20':
            return '#ffa500';
        case '40':
            return '#c63210';
        case '80':
            return '#1ab7cc';
        case '160':
            return '#ae25cc';
        default:
            return "#343434";
    }
}

async function initialize() {
    const map = initMap('map');

    const pointHome = await retrieveHomePoint();
    L.marker(pointHome).addTo(map);

    const pointsHandler = new PointHandler(map);

    new WebSocketClient((latitude, longitude, band) => {
        const point = new L.latLng(latitude, longitude);
        const color = computeColorByBand(band);
        const [marker, geodesic] = generateMarkerGeodesic(pointHome, point, color);
        pointsHandler.addPoint([marker, geodesic]);
    });
}

initialize().then(r => console.log("Initialized"));
