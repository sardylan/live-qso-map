# Contract: HTTP Endpoints (delta to feature 001)

New embedded assets:

| Method | Path | Response |
|--------|------|----------|
| GET | `/view.js` | base map style, projection preference and WebGL check (`text/javascript`) |
| GET | `/layers.js` | GeoJSON for markers and lines (`text/javascript`) |

`index.html` loads MapLibre GL JS 5.24.0 from cdnjs with SRI (research R1) instead of Leaflet:

```html
<link rel="stylesheet" href="https://cdnjs.cloudflare.com/ajax/libs/maplibre-gl/5.24.0/maplibre-gl.css"
      integrity="sha512-KIMsMWIdnoG9OwZa+oaIafmbDonqck1UCmq+/zcUi2aaZ97N9QE6TqpTM+n36EO40HGh64hU/375zxtjx7WTyQ=="
      crossorigin="anonymous" referrerpolicy="no-referrer">
<script src="https://cdnjs.cloudflare.com/ajax/libs/maplibre-gl/5.24.0/maplibre-gl.js"
        integrity="sha512-rOFgoUgWaVi69JW1dzBu6PWF9xKW9rxYfY5u2SexFABHhWk0qUWkJoWNz4UuZRMvSXsNZG66+tU5B+HlwWF5kA=="
        crossorigin="anonymous" referrerpolicy="no-referrer"></script>
```
