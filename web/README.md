# Meater Web Dashboard

Modern web-based dashboard for monitoring Meater probe temperatures in real-time.

## Features

- **Real-time monitoring**: WebSocket connection for instant temperature updates
- **Multi-probe support**: Monitor up to 4 probes simultaneously
- **Beautiful UI**: Modern React dashboard with dark theme
- **Historical data**: View temperature history and trends
- **Battery monitoring**: Track probe battery levels
- **Progress tracking**: Cook time and target temperature progress

## Architecture

- **Backend**: Rust with Axum web framework
- **Frontend**: React with Recharts for data visualization
- **Real-time**: WebSocket for live updates
- **Database**: SQLite for historical data storage

## Running the Server

```bash
# Build and run the web server
cargo run --bin meater-web

# Server will start on http://localhost:3000
```

## API Endpoints

### REST API

- `GET /` - Serve the dashboard HTML
- `GET /api/probes` - Get current probe statuses
- `GET /api/history/:probe_id` - Get historical readings for a probe
- `GET /static/*` - Serve static assets

### WebSocket

- `WS /ws` - Real-time probe event stream

## Dashboard Features

### Probe Cards
Each probe card displays:
- Current tip and ambient temperatures
- Battery level with low-battery warnings
- Target temperature and progress bar
- Mini temperature chart
- Cook time tracking
- Connection status

### Stats Bar
- Active probes count
- Average temperature across all probes
- Connection status (BLE/Offline)
- Overall cooking status

### Real-time Updates
- WebSocket connection for instant updates
- Automatic reconnection on disconnection
- Live temperature graphs

## Development

The frontend uses:
- React 18 (loaded via CDN)
- Recharts for charts (loaded via CDN)
- Tailwind CSS for styling
- Lucide icons for SVG icons

No build step required for the frontend - it uses standalone React with Babel transpilation in the browser.

## Integration

The web server integrates with:
- `ProbeManager` - Multi-probe BLE connection management
- `TemperatureLogger` - Automatic database logging
- Database layer - Historical data queries

## Connection Modes

- **BLE**: Direct Bluetooth connection to probes (default)
- **Offline**: View cached/historical data only

## Future Enhancements

- [ ] Add probe naming/configuration
- [ ] Export data as CSV from web UI
- [ ] Temperature alerts and notifications
- [ ] Custom target temperature per probe
- [ ] Session management from UI
- [ ] Mobile responsive design improvements
- [ ] Production build with Vite/Webpack
