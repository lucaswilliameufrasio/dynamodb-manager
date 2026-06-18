import 'package:flutter/material.dart';
import 'package:flutter/services.dart';
import '../rust/api/dev_logs.dart' as dev_logs;

class DevLogsScreen extends StatefulWidget {
  const DevLogsScreen({super.key});

  @override
  State<DevLogsScreen> createState() => _DevLogsScreenState();
}

class _DevLogsScreenState extends State<DevLogsScreen> {
  List<dev_logs.DevLogEntry> _entries = [];
  bool _loading = true;

  @override
  void initState() {
    super.initState();
    _fetch();
  }

  Future<void> _fetch() async {
    setState(() => _loading = true);
    try {
      _entries = await dev_logs.getRecentDevLogs();
    } catch (_) {}
    if (mounted) setState(() => _loading = false);
  }

  Future<void> _clear() async {
    await dev_logs.clearDevLogs();
    _entries = [];
    if (mounted) setState(() {});
  }

  Color _colorForLevel(String level) {
    switch (level) {
      case 'error':
        return Colors.redAccent;
      case 'warn':
        return Colors.orangeAccent;
      default:
        return Colors.grey;
    }
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: Colors.black,
      appBar: AppBar(
        backgroundColor: Colors.grey.shade900,
        title: const Text('Dev Logs', style: TextStyle(fontSize: 15)),
        actions: [
          IconButton(
            icon: const Icon(Icons.copy, size: 18),
            tooltip: 'Copy all',
            onPressed: _entries.isEmpty
                ? null
                : () {
                    final text = _entries
                        .map(
                          (e) =>
                              '[${e.timestamp}] [${e.level}] [${e.scope}] ${e.message}',
                        )
                        .join('\n');
                    Clipboard.setData(ClipboardData(text: text));
                    ScaffoldMessenger.of(context).showSnackBar(
                      SnackBar(
                        content: Text('Copied ${_entries.length} entries'),
                        duration: const Duration(seconds: 1),
                      ),
                    );
                  },
          ),
          IconButton(
            icon: const Icon(Icons.clear_all, size: 18),
            tooltip: 'Clear logs',
            onPressed: _clear,
          ),
          IconButton(
            icon: const Icon(Icons.refresh, size: 18),
            tooltip: 'Refresh',
            onPressed: _fetch,
          ),
        ],
      ),
      body: _loading
          ? const Center(child: CircularProgressIndicator())
          : _entries.isEmpty
          ? const Center(
              child: Text(
                'No logs yet.\nPerform actions in the app, then refresh.',
                style: TextStyle(color: Colors.grey, fontSize: 13),
                textAlign: TextAlign.center,
              ),
            )
          : ListView.separated(
              padding: const EdgeInsets.all(8),
              itemCount: _entries.length,
              separatorBuilder: (_, _) => const Divider(height: 1),
              itemBuilder: (context, index) {
                final e = _entries[index];
                final color = _colorForLevel(e.level);
                return Padding(
                  padding: const EdgeInsets.symmetric(
                    vertical: 4,
                    horizontal: 4,
                  ),
                  child: Column(
                    crossAxisAlignment: CrossAxisAlignment.start,
                    children: [
                      Row(
                        children: [
                          Container(
                            padding: const EdgeInsets.symmetric(
                              horizontal: 4,
                              vertical: 1,
                            ),
                            decoration: BoxDecoration(
                              color: color.withValues(alpha: 0.15),
                              borderRadius: BorderRadius.circular(3),
                              border: Border.all(color: color, width: 0.5),
                            ),
                            child: Text(
                              e.level.toUpperCase(),
                              style: TextStyle(
                                fontSize: 9,
                                fontWeight: FontWeight.bold,
                                color: color,
                              ),
                            ),
                          ),
                          const SizedBox(width: 6),
                          Text(
                            e.scope,
                            style: TextStyle(
                              fontSize: 11,
                              color: Colors.cyanAccent,
                              fontFamily: 'monospace',
                            ),
                          ),
                          const Spacer(),
                          Text(
                            e.timestamp.length >= 19
                                ? e.timestamp.substring(11, 19)
                                : e.timestamp,
                            style: TextStyle(
                              fontSize: 10,
                              color: Colors.grey.shade600,
                            ),
                          ),
                        ],
                      ),
                      const SizedBox(height: 3),
                      Text(
                        e.message,
                        style: TextStyle(
                          fontSize: 12,
                          color: Colors.grey.shade300,
                          fontFamily: 'monospace',
                        ),
                      ),
                    ],
                  ),
                );
              },
            ),
    );
  }
}
