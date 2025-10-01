const translations = {};
let currentLang = 'en';

/**
 * Loads the translation file for the given language.
 * @param {string} lang - The language code (e.g., 'en', 'zh').
 * @returns {Promise<void>}
 */
async function loadTranslations(lang) {
    try {
        const response = await fetch(`/locales/${lang}.json`);
        if (!response.ok) {
            throw new Error(`Failed to load ${lang}.json`);
        }
        translations[lang] = await response.json();
        console.log(`${lang}.json loaded successfully.`);
    } catch (error) {
        console.error(`[i18n] Error loading translation for ${lang}:`, error);
        // Fallback to English if the desired language fails to load
        if (lang !== 'en') {
            await loadTranslations('en');
        }
    }
}

/**
 * Sets the current language for the application.
 * @param {string} lang - The language code.
 */
async function setLanguage(lang) {
    // If translations for the language are not loaded, load them.
    if (!translations[lang]) {
        await loadTranslations(lang);
    }
    
    // If loading fails, the loadTranslations function might have fallen back.
    // We should only switch if the desired language is actually available.
    if (translations[lang]) {
        currentLang = lang;
        localStorage.setItem('preferredLanguage', lang);
        document.documentElement.lang = lang; // Set the lang attribute on the <html> tag
        updateUI();
    }
}

/**
 * Translates a key into the current language.
 * @param {string} key - The translation key.
 * @returns {string} - The translated string or the key itself if not found.
 */
function t(key) {
    return translations[currentLang]?.[key] || key;
}

/**
 * Updates the entire UI with the current language translations.
 */
function updateUI() {
    if (!translations[currentLang]) {
        console.error(`[i18n] No translations loaded for ${currentLang}.`);
        return;
    }

    document.querySelectorAll('[data-i18n]').forEach(element => {
        const key = element.getAttribute('data-i18n');
        const textElement = element.querySelector('.btn-text, .nav-text'); // Look for any specific text span

        if (textElement) {
            // If the span exists, only update its text
            textElement.textContent = t(key);
        } else {
            // Otherwise, fall back to the original behavior
            element.textContent = t(key);
        }
    });

    document.querySelectorAll('[data-i18n-placeholder]').forEach(element => {
        const key = element.getAttribute('data-i18n-placeholder');
        element.placeholder = t(key);
    });
}

/**
 * Initializes the i18n system.
 * Detects browser language or loads saved preference.
 */
async function initI18n() {
    // Load English first as a fallback
    await loadTranslations('en');

    let preferredLang = localStorage.getItem('preferredLanguage');
    if (!preferredLang) {
        // Detect browser language (e.g., 'en-US', 'zh-CN')
        const browserLang = navigator.language.split('-')[0];
        preferredLang = ['en', 'zh'].includes(browserLang) ? browserLang : 'en';
    }

    // If the preferred language is not English, load it
    if (preferredLang !== 'en') {
        await loadTranslations(preferredLang);
    }

    currentLang = preferredLang;
    document.documentElement.lang = currentLang;
    updateUI();
    
    // Set the language selector dropdown to the current language
    const langSelector = document.getElementById('language-selector');
    if (langSelector) {
        langSelector.value = currentLang;
    }
}

export { t, initI18n, setLanguage };