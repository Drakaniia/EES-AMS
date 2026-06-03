# TODO

- Add optional Google Drive sync for backup files when the app is online.
- Add simple fade-in animation to toast messages.
- Auto-adjust SF2 (output file) rows to fit the full student count.

- Create a manual in PDF.
- View all students, "View All" button.
- In attendance page, the boxes list is too small and the trailing name is three dots (...). Make it big enough to fit the full name without trailing dots.
- Add month in the filename in SF2 export to specify what month it is (ensure month is dynamic based on current month).
- Add validation after importing SF2 with outdated details (e.g., "Report for the month of: March" when the current month is June). Notify or add a modal to update SF2 settings first, pre-filling what can be pre-filled (e.g., current month).
- Add download button in README for Windows.

## Checklist

- Verify behavior when refreshing to a new day.
- Ensure records are maintained until the end of the month.
- In Dashboard, show who is absent, and the count of absent/present students.

- I dont want the bouncing loading. make it simple loader or progress in import sf2 or create workbook loading.
- After Export- should open emedieatly
- In full view. the original sf2 details a feature to hide and appear it, so when hide, all the class day matrix in full view,
- original sf2 header details, and class day matrix,, whats more other term for it.
- Notice that simple smooth transition in sf2 report page when i click the view full Review, apply it to all pages. like when navigationg through all tabs in the sidebar
- Where can we easisy edit the absend or to be present , so even in closed days can still modify if present or absent
- Per day Overview
- Box And List Animation
- Remove the dropdown in section.
- Make it real prograss bar when importing sf2, notr just harcoded or frontend manipulation,
- Validation, should cannot create new sf2 in the settings
- In attendance page. when i click the names in the box. the box grows, when there is recent activity. make it fixed
- Do i need to always end session?what about if thre is incomeing
- dialogue modal for laoding animation when export
- When in the first month. i alrady inputed the data. and the sf2 is now availble, what happenns when i import the sf2 (it makes a working copy) should modify the sf2 with the already recorded (for example there is absend in the first week it auto put x in the day without breaking or deleting the existing record). and the add validation if the names are right or other instances

-Validation if export the sf2 xlsx, the dates should be if whats report of the month of epecially the dates. becaues we create our own copy when importing, it modifies also the exel when import, the months dates in weekly, also include the x mark when absent because currently there is no x mark

- No end session, i should dont have to click end session just to end. when i click attendance in the list should automaticall record
- Automatically close session when the days ends to record automatically

- page where can eassisy modify attendance absent or not with ignoring the class houurs
