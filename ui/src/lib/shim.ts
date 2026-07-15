// Mount shim shared by every window.
//
// The original gpgui pages laid header/main/footer directly under the flex
// <body> (settings/manager additionally styled off <body class="settings">);
// main.ts mounts into <div id="app">, so restore the body class and make the
// #app wrapper transparent to layout for identical rendering with the
// unchanged theme.css.
export function mountShim(bodyClass?: string): void {
  if (bodyClass !== undefined) document.body.classList.add(bodyClass);
  document.getElementById('app')?.style.setProperty('display', 'contents');
}
