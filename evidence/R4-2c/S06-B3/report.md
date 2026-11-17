| scenario | what | result | detail |
|---|---|---|---|
| lat | gpu7 (0269): input latency (the page's own LatencyTracker records) in four situations: (a) idle File-menu clicks x35, (c) a drag of the Mixer header (2 x 40 moves), then 6 s of the same drag for frame/s (dragMeasure c-mixer-fps; added by R4-2), (d) Space x32 with the reference project, (b) File-menu clicks x35 during playback [0271: Firefox; the project is opened as a zip through the Files window (openZipProject) because the 49-file upload aborts in Firefox] | ok | cycles 35/35, popup surface 35 |
