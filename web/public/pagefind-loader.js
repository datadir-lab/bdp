// Pagefind loader - loads Pagefind as a module and exposes it to window
(async function() {
  try {
    const pagefind = await import('/_pagefind/pagefind.js');
    window.pagefind = pagefind;
    console.log('Pagefind loaded and attached to window');
  } catch (error) {
    console.error('Failed to load Pagefind:', error);
  }
})();
