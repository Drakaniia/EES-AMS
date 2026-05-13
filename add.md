Set the desktop app default window size to:

"width": 1500,
"height": 900

This should also be the minimum allowed window size to keep the UI fully responsive and prevent layout breaking on smaller dimensions.

Requirements:
- Users can resize larger than 1500x900
- Users can maximize the window
- Users can use fullscreen mode
- Users must NOT be able to resize below 1500x900
- The UI should remain responsive and stable on larger resolutions
- Prevent shrinking that causes overlapping, clipping, or broken layouts

Behavior:
- Default launch size: 1500x900
- Minimum resize limit: 1500x900
- No maximum size restrictions