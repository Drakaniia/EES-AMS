To make the dashboard in `Overview page` feel truly dynamic and session-aware, you should shift from a "global" overview to a "contextual" one.

Here are specific UI and text suggestions to make the dashboard adapt to the teacher’s schedule:

### 1. Dynamic Headings & Context

Instead of a static "Attendance Overview," use the header to tell the teacher exactly where they are in their workday.

- **Text Suggestion (During Class):**
- **Primary:** "Currently Teaching: **[Subject Code/Name]**"
- **Secondary:** "Room [Room Number] • [Start Time] – [End Time]"

- **Text Suggestion (Before Class starts):**
- **Primary:** "Welcome back, **[Teacher Name]**."
- **Secondary:** "Your next session, **[Class Name]**, begins in **[Minutes]** minutes."

### 2. Session-Specific Stats

In `Overview page`, the cards like "Students Enrolled" are generic. Since a teacher has different schedules, these should update based on the _active_ class roster.

- **UI Change:** Add a "Live Session" indicator (a small pulsing green dot) next to the class name to show that attendance is currently being tracked.
- **Card Update:** Ensure the **"Students Enrolled"** count dynamically switches to the total number of students in the specific section currently being taught.

### 3. The "No Classes" (Idle) State

When there are no classes currently scheduled, the dashboard shouldn't just show "0s" (which can look like an error or a broken system). Use this space to help the teacher prepare.

- **The Default View:**
- **Text:** "No active sessions at the moment."
- **Call to Action:** Display a **"View Full Schedule"** button or a small list of "Upcoming Today" sessions.

- **Visual Change:** Replace the "Currently in the room" empty state with a "Relax" or "Preparation" illustration. You could show a preview of the next class's roster so the teacher can review names beforehand.

### 4. Text Improvements for Scannability

Based on the current layout, some labels could be more descriptive:

| Current Text            | Suggested Text       | Reason                                                       |
| ----------------------- | -------------------- | ------------------------------------------------------------ |
| **Attendance Overview** | **Dashboard**        | Shorter; lets the sub-header handle the context.             |
| **Logged Today**        | **Total Attendance** | "Logged" is a bit technical; "Attendance" is more academic.  |
| **Open Tap Mode**       | **Start Attendance** | Makes the action clearer for a teacher ready to start class. |
| **Students Enrolled**   | **Class Size**       | More concise for a dashboard card.                           |

---
