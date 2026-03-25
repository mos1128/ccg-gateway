# Frontend Redesign Spec: CCG Gateway (Friendly & Professional)

## 1. Overview
The goal is to redesign the frontend of the CCG Gateway Tauri application. The new design must strike a balance between a "$20/month premium tool" and an inviting, user-friendly interface. It completely replaces the generic Element Plus "admin panel" look with a bespoke, vibrant, and airy aesthetic.

## 2. Design Language & Aesthetics
*   **Base Style:** Friendly Ethereal Frost.
*   **Backgrounds:** A soft, airy blue-grey (`#f4f7fe`) for the main app background to reduce eye strain, paired with pure white (`#ffffff`) for content cards.
*   **Colors (Vibrant & Approachable):**
    *   **Primary/Accent:** Bright Ocean Blue (`#0ea5e9`).
    *   **Success:** Emerald Green (`#10b981`).
    *   **Warning:** Amber (`#f59e0b`).
    *   **Danger:** Rose Red (`#f43f5e`).
    *   **Text:** Deep slate (`#0f172a`) for headings, softer slate (`#475569`) for body text.
*   **Shapes & Shadows:** 
    *   Larger, friendlier border-radiuses (`16px` for main cards, `8px` for buttons/inputs).
    *   Soft, diffused drop shadows that give cards a "floating" feel.
*   **Interaction:** Smooth, bouncy transitions (like iOS).

## 3. Structural Changes (Navigation)
*   **Architecture:** Flat with Groups. (Side navigation remains, but styled cleanly without heavy borders).
*   `OVERVIEW`: Dashboard, Sessions, Logs
*   `RESOURCES`: Providers, MCP, Prompts, Skills, Plugins
*   `SYSTEM`: Global Config

## 4. Key View Overhauls

### 4.1 Dashboard (The Control Center)
The dashboard is the most critical view and must provide immediate value and control.
*   **CLI Status Cards (Top Row):**
    *   Redesigned to be the primary control surface.
    *   Removed redundant text ("运行中" / "已停止"), relying solely on the colored status dot for cleaner UI.
    *   Integrated **"Proxy vs. Direct Mode" Segmented Control** directly inside the card. The segmented control has no border, using a slightly darker grey background (`#e2e8f0`) to frame the pure white active pill.
    *   Integrated friendly iOS-style toggle switches for the master Enabled/Disabled state.
*   **KPI Overview (Middle Row):**
    *   4 distinct cards (Requests, Success Rate, Tokens, Active Providers).
    *   **Clean Typography Only:** Icons removed. Focus is entirely on the large, bold numbers and concise labels.
    *   Primary metric uses accent blue; success rate uses emerald green.
*   **Data & Charts (Bottom Row):**
    *   Tables lose their borders and zebra-striping in favor of clean white space and padded rows.
    *   Charts use corresponding vibrant colors (Accent Blue for success, Rose for failure) to match the global theme.

### 5. Implementation Strategy
1.  **CSS Foundation:** Define all custom CSS variables (`--bg-app`, `--accent-blue`, etc.) in a global stylesheet.
2.  **Element Plus Overrides:** Aggressively override Element Plus variables to match the new border-radiuses, shadows, and vibrant colors. Remove all default borders from `el-card` and `el-table`.
3.  **Dashboard Rewrite:** Rewrite `frontend/src/views/dashboard/index.vue` to match the new HTML/CSS mockup (implementing the custom segmented controls for mode switching).
4.  **Global Rollout:** Apply the new card and table styles to Logs, Sessions, Providers, and MCP views.

## 6. Success Criteria
*   The application feels "expensive" but approachable (Notion/Vercel vibes).
*   Users can switch between Proxy and Direct modes directly from the Dashboard.
*   The UI feels alive with smooth, bouncy hover states and vibrant colors.