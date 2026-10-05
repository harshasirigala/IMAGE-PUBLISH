// Frontend configuration
// Automatically detects API endpoint based on current window location
// For Docker: API will be at same hostname as frontend
// For local dev: API at http://127.0.0.1:8080

const API_CONFIG = {
    // Use relative paths or detect from window.location
    // This works whether frontend is served from same origin or different
    getApiUrl: function() {
        // If on localhost:3000 (or any frontend port), backend is usually on same host
        if (window.location.hostname === 'localhost' || window.location.hostname === '127.0.0.1') {
            // Dev mode: assume backend on port 8080
            return `http://${window.location.hostname}:8080`;
        } else {
            // Production: assume same host
            return `http://${window.location.hostname}`;
        }
    }
};

// Export for use in HTML
const API = API_CONFIG.getApiUrl();
