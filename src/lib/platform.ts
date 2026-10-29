/**
 * Which desktop the app is running on. The Windows build renders in WebView2, the Linux
 * build in WebKitGTK; both report the OS in the user agent.
 */
export const isWindows = typeof navigator !== 'undefined' && /Windows/i.test(navigator.userAgent);

/** What the OS file manager is called in tooltips. */
export const fileManagerName = isWindows ? 'Explorer' : 'file manager';
