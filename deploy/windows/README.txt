live-qso-map - a live map of the QSOs your station makes during a contest
=========================================================================

1. Start
   Double-click live-qso-map.exe. A console window opens and the map starts.
   Close the window (or press Ctrl+C in it) to stop the program.
   If the program stops with an error, the message stays on screen until you press Enter.

2. Open the map
   In a browser on this computer, open http://localhost:8080/
   (from another computer, use this computer's address instead of localhost).
   Point your logger's UDP output to this computer, port 12060.

3. Settings
   The program reads live-qso-map.toml from its own folder when that file exists.
   config.example.toml, in this folder, lists every setting, commented out: copy it to
   live-qso-map.toml, uncomment the lines you change and replace the <...> placeholders
   with your own values.
   Or let the program write the file, for example from a terminal in this folder:

       live-qso-map.exe --station-callsign IS0MZK --station-locator JM49NH --save-config live-qso-map.toml

   Every setting is written with its current value; edit the file afterwards if you like.
   It can hold provider passwords (QRZ.com, HamQTH, QRZCQ): keep it private.
   All options are listed by:  live-qso-map.exe --help

4. Country data
   The country data file cty.dat is downloaded into this same folder and refreshed
   every 30 days. Everything the program needs stays in this folder.

License: GPL-3.0, see LICENSE.md. Source: https://github.com/sardylan/live-qso-map
