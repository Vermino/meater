const { useState, useEffect } = React;
const { LineChart, Line, XAxis, YAxis, CartesianGrid, Tooltip, ResponsiveContainer, Area, AreaChart } = window.Recharts || {};

// Check if Recharts is loaded
if (!window.Recharts) {
    console.error('Recharts not loaded! Waiting...');
    setTimeout(() => window.location.reload(), 1000);
}

// Lucide icons as React components (simplified)
const Flame = ({ className }) => (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M12 2C6.48 2 2 6.48 2 12s4.48 10 10 10 10-4.48 10-10S17.52 2 12 2zm0 18c-4.42 0-8-3.58-8-8s3.58-8 8-8 8 3.58 8 8-3.58 8-8 8z" />
    </svg>
);

const ThermometerSun = ({ className }) => (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <circle cx="12" cy="12" r="10" strokeWidth={2} />
    </svg>
);

const Battery = ({ className }) => (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <rect x="2" y="7" width="18" height="10" rx="2" strokeWidth={2} />
        <path d="M22 11v2" strokeWidth={2} strokeLinecap="round" />
    </svg>
);

const BatteryLow = Battery;

const Clock = ({ className }) => (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <circle cx="12" cy="12" r="10" strokeWidth={2} />
        <path d="M12 6v6l4 2" strokeWidth={2} strokeLinecap="round" />
    </svg>
);

const TrendingUp = ({ className }) => (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path d="M22 7l-8.5 8.5-5-5L2 17" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" />
    </svg>
);

const Plus = ({ className }) => (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <line x1="12" y1="5" x2="12" y2="19" strokeWidth={2} strokeLinecap="round" />
        <line x1="5" y1="12" x2="19" y2="12" strokeWidth={2} strokeLinecap="round" />
    </svg>
);

const Radio = ({ className }) => (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <circle cx="12" cy="12" r="3" strokeWidth={2} fill="currentColor" />
        <circle cx="12" cy="12" r="10" strokeWidth={2} />
    </svg>
);

const Settings = ({ className }) => (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <circle cx="12" cy="12" r="3" strokeWidth={2} />
        <path d="M12 1v6m0 6v10M1 12h6m6 0h10" strokeWidth={2} strokeLinecap="round" />
    </svg>
);

const Bell = ({ className }) => (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <path d="M18 8A6 6 0 0 0 6 8c0 7-3 9-3 9h18s-3-2-3-9" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" />
    </svg>
);

const Check = ({ className }) => (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <polyline points="20 6 9 17 4 12" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" />
    </svg>
);

const AlertCircle = ({ className }) => (
    <svg className={className} fill="none" stroke="currentColor" viewBox="0 0 24 24">
        <circle cx="12" cy="12" r="10" strokeWidth={2} />
        <line x1="12" y1="8" x2="12" y2="12" strokeWidth={2} strokeLinecap="round" />
        <circle cx="12" cy="16" r="1" fill="currentColor" />
    </svg>
);

const MeaterDashboard = () => {
    const [currentTime, setCurrentTime] = useState(new Date());
    const [connectionMode, setConnectionMode] = useState('ble');
    const [probes, setProbes] = useState([]);
    const [wsConnected, setWsConnected] = useState(false);

    // Initialize WebSocket connection
    useEffect(() => {
        // Determine WebSocket URL (works for both Tauri and browser)
        const wsProtocol = window.location.protocol === 'https:' ? 'wss:' : 'ws:';
        const wsHost = window.location.host || 'localhost:3000';
        const wsUrl = `${wsProtocol}//${wsHost}/ws`;

        console.log('Connecting to WebSocket:', wsUrl);
        const ws = new WebSocket(wsUrl);

        ws.onopen = () => {
            console.log('WebSocket connected');
            setWsConnected(true);
        };

        ws.onmessage = (event) => {
            const data = JSON.parse(event.data);
            if (data.probes) {
                setProbes(data.probes);
            }
        };

        ws.onclose = () => {
            console.log('WebSocket disconnected');
            setWsConnected(false);
            // Auto-reconnect after 3 seconds
            setTimeout(() => {
                console.log('Attempting to reconnect...');
                window.location.reload();
            }, 3000);
        };

        ws.onerror = (error) => {
            console.error('WebSocket error:', error);
        };

        // Fetch initial probe data
        const apiUrl = window.location.origin + '/api/probes';
        fetch(apiUrl)
            .then(res => res.json())
            .then(data => {
                console.log('Initial probe data:', data);
                setProbes(data);
            })
            .catch(err => console.error('Error fetching probes:', err));

        return () => ws.close();
    }, []);

    // Update clock
    useEffect(() => {
        const timer = setInterval(() => setCurrentTime(new Date()), 1000);
        return () => clearInterval(timer);
    }, []);

    const formatTime = (seconds) => {
        const hours = Math.floor(seconds / 3600);
        const minutes = Math.floor((seconds % 3600) / 60);
        return `${hours}h ${minutes}m`;
    };

    const generateHistoricalData = (probe) => {
        const data = [];
        const points = 30;
        const currentTip = probe.tip_temp || 0;

        for (let i = points; i >= 0; i--) {
            const time = new Date(Date.now() - i * 60000);
            data.push({
                time: time.toLocaleTimeString('en-US', { hour: '2-digit', minute: '2-digit' }),
                tip: Math.max(0, currentTip - (i * 0.5)),
                ambient: probe.ambient_temp || 0,
                target: probe.target_temp || 0
            });
        }
        return data;
    };

    const ProbeCard = ({ probe }) => {
        const progress = probe.target_temp ? (probe.tip_temp / probe.target_temp) * 100 : 0;
        const isLowBattery = probe.battery < 50;

        if (!probe.active) {
            return (
                <div className="bg-gray-800 rounded-2xl p-6 border-2 border-dashed border-gray-700 flex flex-col items-center justify-center min-h-[320px] hover:border-gray-600 transition-colors cursor-pointer">
                    <Plus className="w-12 h-12 text-gray-600 mb-3" />
                    <p className="text-gray-500 font-medium">Add Probe</p>
                </div>
            );
        }

        const historicalData = generateHistoricalData(probe);

        return (
            <div className="bg-gray-800 rounded-2xl p-6 border border-gray-700 hover:border-gray-600 transition-all shadow-lg">
                <div className="flex items-start justify-between mb-6">
                    <div>
                        <h3 className="text-xl font-bold text-white mb-1">{probe.name}</h3>
                        <div className="flex items-center gap-2">
                            <div className={`w-2 h-2 rounded-full ${probe.status === 'cooking' ? 'bg-green-400 animate-pulse' : 'bg-gray-400'}`}></div>
                            <span className="text-sm text-gray-400 capitalize">{probe.status.replace('-', ' ')}</span>
                        </div>
                    </div>
                    <div className="flex items-center gap-2">
                        {isLowBattery ? (
                            <BatteryLow className="w-5 h-5 text-orange-400" />
                        ) : (
                            <Battery className="w-5 h-5 text-green-400" />
                        )}
                        <span className={`text-sm font-medium ${isLowBattery ? 'text-orange-400' : 'text-green-400'}`}>
                            {probe.battery}%
                        </span>
                    </div>
                </div>

                <div className="grid grid-cols-2 gap-4 mb-6">
                    <div className="bg-gray-900 rounded-xl p-4">
                        <div className="flex items-center gap-2 mb-2">
                            <ThermometerSun className="w-4 h-4 text-red-400" />
                            <span className="text-xs text-gray-400 font-medium">Internal</span>
                        </div>
                        <div className="text-3xl font-bold text-white">{probe.tip_temp.toFixed(1)}°</div>
                        {probe.target_temp && (
                            <div className="text-xs text-gray-500 mt-1">Target: {probe.target_temp.toFixed(0)}°F</div>
                        )}
                    </div>
                    <div className="bg-gray-900 rounded-xl p-4">
                        <div className="flex items-center gap-2 mb-2">
                            <Flame className="w-4 h-4 text-orange-400" />
                            <span className="text-xs text-gray-400 font-medium">Ambient</span>
                        </div>
                        <div className="text-3xl font-bold text-white">{probe.ambient_temp.toFixed(1)}°</div>
                        <div className="text-xs text-gray-500 mt-1">Grill temp</div>
                    </div>
                </div>

                {probe.target_temp && (
                    <div className="mb-6">
                        <div className="flex justify-between text-xs text-gray-400 mb-2">
                            <span>Progress</span>
                            <span>{Math.min(100, progress).toFixed(0)}%</span>
                        </div>
                        <div className="w-full bg-gray-700 rounded-full h-2 overflow-hidden">
                            <div
                                className="h-full rounded-full transition-all duration-1000"
                                style={{
                                    width: `${Math.min(100, progress)}%`,
                                    backgroundColor: probe.color
                                }}
                            ></div>
                        </div>
                    </div>
                )}

                <div className="mb-4 -mx-2">
                    <ResponsiveContainer width="100%" height={80}>
                        <AreaChart data={historicalData}>
                            <defs>
                                <linearGradient id={`gradient-${probe.id}`} x1="0" y1="0" x2="0" y2="1">
                                    <stop offset="5%" stopColor={probe.color} stopOpacity={0.3}/>
                                    <stop offset="95%" stopColor={probe.color} stopOpacity={0}/>
                                </linearGradient>
                            </defs>
                            <Area
                                type="monotone"
                                dataKey="tip"
                                stroke={probe.color}
                                fill={`url(#gradient-${probe.id})`}
                                strokeWidth={2}
                            />
                        </AreaChart>
                    </ResponsiveContainer>
                </div>

                {probe.cook_time > 0 && (
                    <div className="grid grid-cols-2 gap-3">
                        <div className="flex items-center gap-2 text-sm">
                            <Clock className="w-4 h-4 text-gray-400" />
                            <div>
                                <div className="text-gray-500 text-xs">Cook Time</div>
                                <div className="text-white font-medium">{formatTime(probe.cook_time)}</div>
                            </div>
                        </div>
                        <div className="flex items-center gap-2 text-sm">
                            <TrendingUp className="w-4 h-4 text-gray-400" />
                            <div>
                                <div className="text-gray-500 text-xs">Status</div>
                                <div className="text-white font-medium capitalize">{probe.status}</div>
                            </div>
                        </div>
                    </div>
                )}
            </div>
        );
    };

    const ConnectionBadge = () => {
        const modes = {
            ble: { icon: Radio, text: 'BLE Connected', color: 'bg-green-500' },
            offline: { icon: Radio, text: 'Offline', color: 'bg-gray-500' }
        };

        const { icon: Icon, text, color } = modes[connectionMode] || modes.offline;

        return (
            <div className={`flex items-center gap-2 px-3 py-1.5 rounded-full ${color} bg-opacity-20`}>
                <Icon className={`w-4 h-4 ${color.replace('bg-', 'text-')}`} />
                <span className={`text-sm font-medium ${color.replace('bg-', 'text-')}`}>
                    {wsConnected ? text : 'Connecting...'}
                </span>
            </div>
        );
    };

    const activeProbes = probes.filter(p => p.active);
    const avgTemp = activeProbes.length > 0
        ? activeProbes.reduce((acc, p) => acc + p.tip_temp, 0) / activeProbes.length
        : 0;

    return (
        <div className="min-h-screen bg-gray-900 p-6">
            <div className="max-w-7xl mx-auto">
                <div className="flex items-center justify-between mb-8">
                    <div>
                        <div className="flex items-center gap-3 mb-2">
                            <div className="bg-gradient-to-r from-red-500 to-orange-500 p-3 rounded-xl">
                                <Flame className="w-8 h-8 text-white" />
                            </div>
                            <div>
                                <h1 className="text-3xl font-bold text-white">Meater Monitor</h1>
                                <p className="text-gray-400">Real-time BBQ temperature tracking</p>
                            </div>
                        </div>
                    </div>
                    <div className="flex items-center gap-4">
                        <ConnectionBadge />
                        <button
                            onClick={() => setConnectionMode(connectionMode === 'ble' ? 'offline' : 'ble')}
                            className="p-2 bg-gray-800 rounded-lg border border-gray-700 hover:bg-gray-700 transition-colors"
                        >
                            <Settings className="w-5 h-5 text-gray-400" />
                        </button>
                    </div>
                </div>

                <div className="grid grid-cols-4 gap-4 mb-6">
                    <div className="bg-gray-800 rounded-xl p-4 border border-gray-700">
                        <div className="text-gray-400 text-sm mb-1">Active Probes</div>
                        <div className="text-2xl font-bold text-white">{activeProbes.length}/4</div>
                    </div>
                    <div className="bg-gray-800 rounded-xl p-4 border border-gray-700">
                        <div className="text-gray-400 text-sm mb-1">Avg Temp</div>
                        <div className="text-2xl font-bold text-white">{avgTemp.toFixed(0)}°F</div>
                    </div>
                    <div className="bg-gray-800 rounded-xl p-4 border border-gray-700">
                        <div className="text-gray-400 text-sm mb-1">Connection</div>
                        <div className={`text-2xl font-bold ${wsConnected ? 'text-green-400' : 'text-gray-400'}`}>
                            {wsConnected ? 'Live' : 'Offline'}
                        </div>
                    </div>
                    <div className="bg-gray-800 rounded-xl p-4 border border-gray-700">
                        <div className="text-gray-400 text-sm mb-1">Status</div>
                        <div className="text-2xl font-bold text-green-400">
                            {activeProbes.length > 0 ? 'Cooking' : 'Idle'}
                        </div>
                    </div>
                </div>

                <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 mb-6">
                    {probes.map(probe => (
                        <ProbeCard key={probe.id} probe={probe} />
                    ))}
                </div>

                <div className="mt-8 text-center text-gray-500 text-sm">
                    <p>Last updated: {currentTime.toLocaleTimeString()}</p>
                </div>
            </div>
        </div>
    );
};

const root = ReactDOM.createRoot(document.getElementById('root'));
root.render(<MeaterDashboard />);
