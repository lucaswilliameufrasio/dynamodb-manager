import 'package:flutter/material.dart';
import '../models/dynamo_item.dart';
import '../rust/api/aws_profiles.dart' as profiles;
import '../controllers/workspace_controller.dart';
import 'dev_logs_screen.dart';

// ═══════════════════════════════════════════════════════════════════════════
// PROFILE SELECTION SCREEN
// ═══════════════════════════════════════════════════════════════════════════

class ProfileSelectionScreen extends StatefulWidget {
  final WorkspaceController controller;

  const ProfileSelectionScreen({super.key, required this.controller});

  @override
  State<ProfileSelectionScreen> createState() => _ProfileSelectionScreenState();
}

class _ProfileSelectionScreenState extends State<ProfileSelectionScreen> {
  List<profiles.AwsProfile> _profileList = [];
  List<String> _capabilities = [];
  profiles.AwsDiagnostics? _diagnostics;
  bool _loadingProfiles = true;
  bool _loadingCaps = true;
  String? _profilesError;
  String? _capsError;
  String? _loginProfile;

  bool get _loading => _loadingProfiles;

  @override
  void initState() {
    super.initState();
    _initData();
  }

  Future<void> _initData() async {
    // Step 1: load profiles first (fast – just file reads)
    setState(() {
      _loadingProfiles = true;
      _loadingCaps = true;
      _profilesError = null;
      _capsError = null;
    });

    try {
      _profileList = await profiles.listLocalAwsProfiles();
      _profilesError = null;
    } catch (e) {
      _profilesError = e.toString();
    }
    if (mounted) setState(() => _loadingProfiles = false);

    // Step 2: load capabilities and diagnostics in background
    _fetchCapabilities();
    _fetchDiagnostics();
  }

  Future<void> _fetchCapabilities() async {
    try {
      _capabilities = await profiles.getAwsCliCapabilities();
      _capsError = null;
    } catch (e) {
      _capsError = e.toString();
      _capabilities = [];
    }
    if (mounted) setState(() => _loadingCaps = false);
  }

  Future<void> _fetchDiagnostics() async {
    try {
      _diagnostics = await profiles.getAwsDiagnostics();
    } catch (_) {}
    if (mounted) setState(() {});
  }

  bool get _hasAwsLogin => _capabilities.contains('aws_login');
  bool get _hasSsoLogin => _capabilities.contains('sso_login');
  bool get _hasConfigureSso => _capabilities.contains('configure_sso');

  Future<void> _doLogin(String profileName, String kind) async {
    setState(() => _loginProfile = profileName);
    if (!mounted) return;

    ScaffoldMessenger.of(context).showSnackBar(
      SnackBar(
        content: Text(kind == 'sso'
            ? 'Opening browser for SSO authentication...'
            : 'Opening browser for AWS console login...'),
        duration: const Duration(seconds: 2),
      ),
    );

    try {
      final result = kind == 'sso'
          ? await profiles.ssoLogin(profileName: profileName)
          : await profiles.awsLogin(profileName: profileName);
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('✓ $result'), backgroundColor: Colors.green),
        );
      }
    } catch (e) {
      if (mounted) {
        ScaffoldMessenger.of(context).showSnackBar(
          SnackBar(content: Text('✗ $e'), backgroundColor: Colors.redAccent),
        );
      }
    }

    if (mounted) setState(() => _loginProfile = null);
  }

  Future<void> _deleteProfile(String name) async {
    final confirm = await showDialog<bool>(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Delete Profile'),
        content: Text('Delete AWS profile "$name"?'),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx, false), child: const Text('Cancel')),
          TextButton(
            onPressed: () => Navigator.pop(ctx, true),
            child: const Text('Delete', style: TextStyle(color: Colors.redAccent)),
          ),
        ],
      ),
    );

    if (confirm == true) {
      try {
        await profiles.deleteProfile(name: name);
        await _loadProfiles();
      } catch (e) {
        if (mounted) {
          ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text('$e')));
        }
      }
    }
  }

  Future<void> _loadProfiles() async {
    try {
      _profileList = await profiles.listLocalAwsProfiles();
      _diagnostics = await profiles.getAwsDiagnostics();
    } catch (e) {
      _profilesError = e.toString();
    }
    if (mounted) setState(() {});
  }

  void _showAddProfileSheet() {
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(16)),
      ),
      builder: (ctx) => _AddProfileSheet(
        hasAwsLogin: _hasAwsLogin,
        hasConfigureSso: _hasConfigureSso,
        onDone: _loadProfiles,
      ),
    );
  }

  void _showDiagnosticsSheet() {
    if (_diagnostics == null) return;
    showDialog(
      context: context,
      builder: (ctx) => AlertDialog(
        title: const Text('Profile Discovery Diagnostics'),
        content: SingleChildScrollView(
          child: Column(
            crossAxisAlignment: CrossAxisAlignment.start,
            mainAxisSize: MainAxisSize.min,
            children: [
              _diagRow('Config path', _diagnostics!.configPath),
              _diagRow('Config exists', _diagnostics!.configExists ? 'yes' : 'no'),
              _diagRow('Credentials path', _diagnostics!.credentialsPath),
              _diagRow('Credentials exists', _diagnostics!.credentialsExists ? 'yes' : 'no'),
              _diagRow('Capabilities', _diagnostics!.capabilities.join(', ')),
              _diagRow('Profiles found', '${_diagnostics!.profileCount}'),
              if (_diagnostics!.errors.isNotEmpty) ...[
                const SizedBox(height: 8),
                const Text('Errors:', style: TextStyle(fontWeight: FontWeight.bold, color: Colors.redAccent)),
                ..._diagnostics!.errors.map((e) => Text('  • $e', style: const TextStyle(fontSize: 12))),
              ],
            ],
          ),
        ),
        actions: [
          TextButton(onPressed: () => Navigator.pop(ctx), child: const Text('Close')),
        ],
      ),
    );
  }

  Widget _diagRow(String label, String value) {
    return Padding(
      padding: const EdgeInsets.symmetric(vertical: 2),
      child: Row(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          SizedBox(
            width: 140,
            child: Text('$label:', style: const TextStyle(fontWeight: FontWeight.w500, fontSize: 12)),
          ),
          Expanded(child: Text(value, style: const TextStyle(fontSize: 12, fontFamily: 'monospace'))),
        ],
      ),
    );
  }

  void _selectProfile(profiles.AwsProfile profile) {
    widget.controller.setProfile(profile.name);
    Navigator.of(context).pushReplacement(
      MaterialPageRoute(
        builder: (_) => WorkspaceScreen(controller: widget.controller),
      ),
    );
  }

  @override
  Widget build(BuildContext context) {
    return Scaffold(
      backgroundColor: Colors.black,
      appBar: AppBar(
        backgroundColor: Colors.grey.shade900,
        title: const Text('AWS Profiles'),
        actions: [
          if (_diagnostics != null)
            IconButton(icon: const Icon(Icons.bug_report, size: 18), onPressed: _showDiagnosticsSheet, tooltip: 'Diagnostics'),
          IconButton(icon: const Icon(Icons.developer_mode, size: 18), onPressed: () => Navigator.push(context, MaterialPageRoute(builder: (_) => const DevLogsScreen())), tooltip: 'Dev Logs'),
          IconButton(icon: const Icon(Icons.refresh), onPressed: _initData),
          IconButton(icon: const Icon(Icons.add), onPressed: _showAddProfileSheet),
        ],
      ),
      body: _buildBody(),
    );
  }

  Widget _buildBody() {
    if (_loading) {
      return const Center(child: CircularProgressIndicator());
    }

    if (_profilesError != null) {
      return Center(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            Text(_profilesError!, style: const TextStyle(color: Colors.redAccent)),
            const SizedBox(height: 16),
            ElevatedButton(onPressed: _initData, child: const Text('Retry')),
          ],
        ),
      );
    }

    return Column(
      children: [
        _buildCapabilitiesBar(),
        Expanded(
          child: _profileList.isEmpty
              ? _buildEmptyState()
              : _buildProfileList(),
        ),
      ],
    );
  }

  Widget _buildCapabilitiesBar() {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 16, vertical: 8),
      color: Colors.grey.shade900,
      child: Row(
        children: [
          const Text('Auth methods:', style: TextStyle(fontSize: 11, color: Colors.grey)),
          const SizedBox(width: 8),
          if (_loadingCaps)
            const SizedBox(
              width: 14, height: 14,
              child: CircularProgressIndicator(strokeWidth: 2),
            )
          else ...[
            _capChip('aws login', _hasAwsLogin),
            const SizedBox(width: 6),
            _capChip('aws sso login', _hasSsoLogin),
            const SizedBox(width: 6),
            _capChip('aws configure sso', _hasConfigureSso),
          ],
          if (_capsError != null) ...[
            const SizedBox(width: 8),
            Tooltip(
              message: _capsError!,
              child: Icon(Icons.warning_amber, size: 14, color: Colors.orangeAccent),
            ),
          ],
        ],
      ),
    );
  }

  Widget _capChip(String label, bool supported) {
    return Container(
      padding: const EdgeInsets.symmetric(horizontal: 6, vertical: 3),
      decoration: BoxDecoration(
        color: supported ? Colors.green.withValues(alpha: 0.12) : Colors.grey.withValues(alpha: 0.12),
        borderRadius: BorderRadius.circular(3),
        border: Border.all(color: supported ? Colors.greenAccent : Colors.grey, width: 0.5),
      ),
      child: Text(
        supported ? '✓ $label' : '✗ $label',
        style: TextStyle(fontSize: 10, color: supported ? Colors.greenAccent : Colors.grey),
      ),
    );
  }

  Widget _buildEmptyState() {
    final reasons = <String>[];
    if (_diagnostics != null) {
      if (!_diagnostics!.credentialsExists) {
        reasons.add('✗ credentials file not found at: ${_diagnostics!.credentialsPath}');
      }
      if (!_diagnostics!.configExists) {
        reasons.add('✗ config file not found at: ${_diagnostics!.configPath}');
      }
      if (!_hasAwsLogin && !_hasSsoLogin) {
        reasons.add('✗ AWS CLI does not support aws login or aws sso login');
      }
    }

    return Center(
      child: Padding(
        padding: const EdgeInsets.all(24),
        child: Column(
          mainAxisSize: MainAxisSize.min,
          children: [
            const Icon(Icons.cloud_off, size: 48, color: Colors.grey),
            const SizedBox(height: 16),
            const Text('No AWS profiles found', style: TextStyle(fontSize: 16, color: Colors.white)),
            const SizedBox(height: 8),
            if (reasons.isNotEmpty)
              ...reasons.map((r) => Padding(
                padding: const EdgeInsets.only(top: 4),
                child: Text(r, style: const TextStyle(fontSize: 11, color: Colors.grey), textAlign: TextAlign.center),
              )),
            const SizedBox(height: 16),
            _actionChip(Icons.terminal, 'Run aws login in terminal', () async {
              final nameC = TextEditingController();
              final name = await showDialog<String>(
                context: context,
                builder: (ctx) => AlertDialog(
                  title: const Text('Profile name'),
                  content: TextField(
                    controller: nameC,
                    decoration: const InputDecoration(
                      hintText: 'default',
                      border: OutlineInputBorder(),
                    ),
                  ),
                  actions: [
                    TextButton(onPressed: () => Navigator.pop(ctx), child: const Text('Cancel')),
                    TextButton(onPressed: () => Navigator.pop(ctx, nameC.text.trim()), child: const Text('Use')),
                  ],
                ),
              );
              nameC.dispose();
              if (name != null && mounted) {
                await _doLogin(name.isEmpty ? 'default' : name, 'static');
                await _loadProfiles();
              }
            }),
            const SizedBox(height: 8),
            _actionChip(Icons.refresh, 'Refresh profiles', _initData),
            const SizedBox(height: 8),
            if (_diagnostics != null)
              _actionChip(Icons.bug_report, 'View diagnostics', _showDiagnosticsSheet),
          ],
        ),
      ),
    );
  }

  Widget _actionChip(IconData icon, String label, VoidCallback onTap) {
    return SizedBox(
      width: 260,
      child: OutlinedButton.icon(
        onPressed: onTap,
        icon: Icon(icon, size: 16),
        label: Text(label, style: const TextStyle(fontSize: 12)),
        style: OutlinedButton.styleFrom(
          foregroundColor: Colors.grey.shade300,
          side: BorderSide(color: Colors.grey.shade700),
        ),
      ),
    );
  }

  Widget _buildProfileList() {
    return ListView.separated(
      padding: const EdgeInsets.all(16),
      itemCount: _profileList.length,
      separatorBuilder: (_, _) => const Divider(height: 1, color: Colors.grey),
      itemBuilder: (context, index) {
        final p = _profileList[index];
        final isLoggingIn = _loginProfile == p.name;

        return ListTile(
          leading: Icon(
            _profileIcon(p.kind),
            color: _profileColor(p.kind),
          ),
          title: Text(p.name, style: const TextStyle(color: Colors.white)),
          subtitle: Text(
            _profileKindLabel(p.kind),
            style: TextStyle(color: Colors.grey.shade500, fontSize: 12),
          ),
          trailing: Row(
            mainAxisSize: MainAxisSize.min,
            children: [
              if (isLoggingIn)
                const SizedBox(width: 20, height: 20, child: CircularProgressIndicator(strokeWidth: 2))
              else if (p.kind == 'sso' && _hasSsoLogin)
                TextButton(
                  onPressed: () => _doLogin(p.name, p.kind),
                  child: const Text('Login', style: TextStyle(color: Colors.orangeAccent)),
                )
              else if (p.kind != 'sso' && _hasAwsLogin)
                TextButton(
                  onPressed: () => _doLogin(p.name, p.kind),
                  child: const Text('Login', style: TextStyle(color: Colors.lightBlueAccent)),
                ),
              PopupMenuButton<String>(
                onSelected: (action) {
                  if (action == 'delete') _deleteProfile(p.name);
                },
                itemBuilder: (_) => [
                  const PopupMenuItem(value: 'delete', child: Text('Delete'))
                ],
              ),
            ],
          ),
          onTap: isLoggingIn ? null : () => _selectProfile(p),
        );
      },
    );
  }

  IconData _profileIcon(String kind) {
    switch (kind) {
      case 'sso': return Icons.shield_outlined;
      case 'role': return Icons.swap_horiz;
      case 'credential_source': return Icons.cloud_outlined;
      case 'short_term': return Icons.timer_outlined;
      case 'static': return Icons.vpn_key_outlined;
      default: return Icons.person_outline;
    }
  }

  Color _profileColor(String kind) {
    switch (kind) {
      case 'sso': return Colors.orangeAccent;
      case 'role': return Colors.purpleAccent;
      case 'credential_source': return Colors.tealAccent;
      case 'short_term': return Colors.cyanAccent;
      case 'static': return Colors.greenAccent;
      default: return Colors.grey;
    }
  }

  String _profileKindLabel(String kind) {
    switch (kind) {
      case 'sso': return 'IAM Identity Center';
      case 'role': return 'IAM Role';
      case 'credential_source': return 'Credential Source';
      case 'short_term': return 'Temporary Credentials';
      case 'static': return 'Access Key';
      default: return kind;
    }
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// ADD PROFILE BOTTOM SHEET
// ═══════════════════════════════════════════════════════════════════════════

class _AddProfileSheet extends StatefulWidget {
  final bool hasAwsLogin;
  final bool hasConfigureSso;
  final VoidCallback onDone;

  const _AddProfileSheet({
    required this.hasAwsLogin,
    required this.hasConfigureSso,
    required this.onDone,
  });

  @override
  State<_AddProfileSheet> createState() => _AddProfileSheetState();
}

class _AddProfileSheetState extends State<_AddProfileSheet> {
  @override
  Widget build(BuildContext context) {
    final bool loginAvailable = widget.hasAwsLogin;
    final bool ssoAvailable = widget.hasConfigureSso;

    return Padding(
      padding: EdgeInsets.only(
        left: 16, right: 16, top: 16,
        bottom: MediaQuery.of(context).viewInsets.bottom + 16,
      ),
      child: Column(
        mainAxisSize: MainAxisSize.min,
        crossAxisAlignment: CrossAxisAlignment.stretch,
        children: [
          const Text('Add Profile', style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold)),
          const SizedBox(height: 16),
          _OptionTile(
            icon: Icons.login,
            title: loginAvailable ? 'Login with AWS Console' : 'Login with AWS Console (not available)',
            subtitle: loginAvailable
                ? 'Opens browser to sign in with your AWS Console session'
                : 'AWS CLI does not support `aws login` on this machine',
            color: loginAvailable ? Colors.lightBlueAccent : Colors.grey,
            enabled: loginAvailable,
            onTap: loginAvailable ? _showLoginSheet : null,
          ),
          const SizedBox(height: 8),
          _OptionTile(
            icon: Icons.shield_outlined,
            title: ssoAvailable ? 'Configure IAM Identity Center' : 'Configure IAM Identity Center (not available)',
            subtitle: ssoAvailable
                ? 'Set up an SSO profile with start URL, region, account, role'
                : 'AWS CLI does not support `aws configure sso` on this machine',
            color: ssoAvailable ? Colors.orangeAccent : Colors.grey,
            enabled: ssoAvailable,
            onTap: ssoAvailable ? _showSsoForm : null,
          ),
          const SizedBox(height: 8),
          _OptionTile(
            icon: Icons.refresh,
            title: 'Refresh & detect existing profiles',
            subtitle: 'Re-scan credential files for profiles you created via CLI',
            color: Colors.grey,
            enabled: true,
            onTap: () { Navigator.pop(context); widget.onDone(); },
          ),
        ],
      ),
    );
  }

  void _showLoginSheet() {
    Navigator.pop(context);
    showDialog(
      context: context,
      builder: (ctx) => _LoginProfileDialog(onDone: widget.onDone),
    );
  }

  void _showSsoForm() {
    Navigator.pop(context);
    showModalBottomSheet(
      context: context,
      isScrollControlled: true,
      shape: const RoundedRectangleBorder(
        borderRadius: BorderRadius.vertical(top: Radius.circular(16)),
      ),
      builder: (ctx) => _SsoFormSheet(onDone: widget.onDone),
    );
  }
}

class _OptionTile extends StatelessWidget {
  final IconData icon;
  final String title;
  final String subtitle;
  final Color color;
  final bool enabled;
  final VoidCallback? onTap;

  const _OptionTile({
    required this.icon,
    required this.title,
    required this.subtitle,
    required this.color,
    required this.enabled,
    this.onTap,
  });

  @override
  Widget build(BuildContext context) {
    return Card(
      color: Colors.grey.shade800,
      child: Opacity(
        opacity: enabled ? 1.0 : 0.5,
        child: ListTile(
          leading: Icon(icon, color: color),
          title: Text(title, style: const TextStyle(fontWeight: FontWeight.w500, fontSize: 14)),
          subtitle: Text(subtitle, style: const TextStyle(fontSize: 11, color: Colors.grey)),
          trailing: enabled ? const Icon(Icons.chevron_right) : const Icon(Icons.block, size: 16, color: Colors.grey),
          onTap: enabled ? onTap : null,
        ),
      ),
    );
  }
}

// ─── Login Profile Dialog ──────────────────────────────────────────────────

class _LoginProfileDialog extends StatefulWidget {
  final VoidCallback onDone;
  const _LoginProfileDialog({required this.onDone});

  @override
  State<_LoginProfileDialog> createState() => _LoginProfileDialogState();
}

class _LoginProfileDialogState extends State<_LoginProfileDialog> {
  final _nameCtrl = TextEditingController();
  bool _busy = false;
  String? _error;

  @override
  void dispose() {
    _nameCtrl.dispose();
    super.dispose();
  }

  Future<void> _login() async {
    final name = _nameCtrl.text.trim();
    if (name.isEmpty) {
      setState(() => _error = 'Profile name is required.');
      return;
    }
    setState(() { _busy = true; _error = null; });
    try {
      await profiles.awsLogin(profileName: name);
      if (mounted) Navigator.pop(context);
      widget.onDone();
    } catch (e) {
      setState(() => _error = e.toString());
    }
    setState(() => _busy = false);
  }

  @override
  Widget build(BuildContext context) {
    return AlertDialog(
      title: const Text('Login with AWS Console'),
      content: Column(
        mainAxisSize: MainAxisSize.min,
        children: [
          TextField(
            controller: _nameCtrl,
            decoration: const InputDecoration(
              labelText: 'Profile Name',
              hintText: 'Leave empty for default',
              border: OutlineInputBorder(),
            ),
          ),
          if (_error != null)
            Padding(
              padding: const EdgeInsets.only(top: 12),
              child: Text(_error!, style: const TextStyle(color: Colors.redAccent)),
            ),
        ],
      ),
      actions: [
        TextButton(onPressed: () => Navigator.pop(context), child: const Text('Cancel')),
        FilledButton(
          onPressed: _busy ? null : _login,
          child: _busy
              ? const SizedBox(width: 20, height: 20, child: CircularProgressIndicator(strokeWidth: 2))
              : const Text('Login'),
        ),
      ],
    );
  }
}

// ─── SSO Form Sheet ────────────────────────────────────────────────────────

class _SsoFormSheet extends StatefulWidget {
  final VoidCallback onDone;
  const _SsoFormSheet({required this.onDone});

  @override
  State<_SsoFormSheet> createState() => _SsoFormSheetState();
}

class _SsoFormSheetState extends State<_SsoFormSheet> {
  bool _saving = false;
  String? _error;

  final _nameCtrl = TextEditingController();
  final _sessionCtrl = TextEditingController();
  final _startUrlCtrl = TextEditingController();
  final _ssoRegionCtrl = TextEditingController();
  final _accountCtrl = TextEditingController();
  final _roleCtrl = TextEditingController();

  @override
  void dispose() {
    _nameCtrl.dispose();
    _sessionCtrl.dispose();
    _startUrlCtrl.dispose();
    _ssoRegionCtrl.dispose();
    _accountCtrl.dispose();
    _roleCtrl.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    final name = _nameCtrl.text.trim();
    if (name.isEmpty) {
      setState(() => _error = 'Profile name is required.');
      return;
    }
    setState(() { _saving = true; _error = null; });

    try {
      await profiles.addSsoProfile(
        name: name,
        ssoSession: _sessionCtrl.text.trim().isNotEmpty ? _sessionCtrl.text.trim() : name,
        ssoStartUrl: _startUrlCtrl.text.trim(),
        ssoRegion: _ssoRegionCtrl.text.trim(),
        ssoAccountId: _accountCtrl.text.trim(),
        ssoRoleName: _roleCtrl.text.trim(),
      );
      if (mounted) Navigator.pop(context);
      widget.onDone();
    } catch (e) {
      setState(() => _error = e.toString());
    }
    setState(() => _saving = false);
  }

  @override
  Widget build(BuildContext context) {
    return Padding(
      padding: EdgeInsets.only(
        left: 16, right: 16, top: 16,
        bottom: MediaQuery.of(context).viewInsets.bottom + 16,
      ),
      child: SingleChildScrollView(
        child: Column(
          mainAxisSize: MainAxisSize.min,
          crossAxisAlignment: CrossAxisAlignment.stretch,
          children: [
            const Text('Configure IAM Identity Center', style: TextStyle(fontSize: 18, fontWeight: FontWeight.bold)),
            const SizedBox(height: 16),
            TextField(
              controller: _nameCtrl,
              decoration: const InputDecoration(
                labelText: 'Profile Name', hintText: 'e.g. my-sso-profile', border: OutlineInputBorder(),
              ),
            ),
            const SizedBox(height: 8),
            TextField(
              controller: _sessionCtrl,
              decoration: const InputDecoration(
                labelText: 'SSO Session Name', hintText: 'Leave empty to use profile name', border: OutlineInputBorder(),
              ),
            ),
            const SizedBox(height: 8),
            TextField(
              controller: _startUrlCtrl,
              decoration: const InputDecoration(
                labelText: 'SSO Start URL', hintText: 'https://my-sso-portal.awsapps.com/start', border: OutlineInputBorder(),
              ),
            ),
            const SizedBox(height: 8),
            TextField(
              controller: _ssoRegionCtrl,
              decoration: const InputDecoration(
                labelText: 'SSO Region', hintText: 'us-east-1', border: OutlineInputBorder(),
              ),
            ),
            const SizedBox(height: 8),
            TextField(
              controller: _accountCtrl,
              decoration: const InputDecoration(
                labelText: 'SSO Account ID', hintText: '123456789012', border: OutlineInputBorder(),
              ),
            ),
            const SizedBox(height: 8),
            TextField(
              controller: _roleCtrl,
              decoration: const InputDecoration(
                labelText: 'SSO Role Name', hintText: 'AdministratorAccess', border: OutlineInputBorder(),
              ),
            ),
            if (_error != null)
              Padding(
                padding: const EdgeInsets.only(top: 8),
                child: Text(_error!, style: const TextStyle(color: Colors.redAccent)),
              ),
            const SizedBox(height: 16),
            Row(
              children: [
                Expanded(child: OutlinedButton(onPressed: () => Navigator.pop(context), child: const Text('Cancel'))),
                const SizedBox(width: 12),
                Expanded(
                  flex: 2,
                  child: FilledButton(
                    onPressed: _saving ? null : _save,
                    child: _saving
                        ? const SizedBox(width: 20, height: 20, child: CircularProgressIndicator(strokeWidth: 2))
                        : const Text('Save & Login Later'),
                  ),
                ),
              ],
            ),
          ],
        ),
      ),
    );
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// WORKSPACE SCREEN
// ═══════════════════════════════════════════════════════════════════════════

class WorkspaceScreen extends StatefulWidget {
  final WorkspaceController controller;
  const WorkspaceScreen({super.key, required this.controller});

  @override
  State<WorkspaceScreen> createState() => _WorkspaceScreenState();
}

class _WorkspaceScreenState extends State<WorkspaceScreen> {
  @override
  void initState() {
    super.initState();
    widget.controller.loadTables();
  }

  @override
  Widget build(BuildContext context) {
    final c = widget.controller;
    return ListenableBuilder(
      listenable: c,
      builder: (context, _) {
        return Scaffold(
          body: Row(
            children: [
              SizedBox(width: c.sidebarWidth, child: _SidebarPane(controller: c)),
              _ResizableDivider(onDrag: (details) => c.resizeSidebar(details.delta.dx)),
              Expanded(child: _MainWorkspacePane(controller: c)),
            ],
          ),
        );
      },
    );
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// SIDEBAR
// ═══════════════════════════════════════════════════════════════════════════

class _SidebarPane extends StatelessWidget {
  final WorkspaceController controller;
  const _SidebarPane({required this.controller});

  @override
  Widget build(BuildContext context) {
    return Column(
      crossAxisAlignment: CrossAxisAlignment.stretch,
      children: [
        Container(
          padding: const EdgeInsets.all(12),
          color: Colors.grey.shade900,
          child: Row(
            children: [
              Expanded(
                child: Text(controller.profile, style: const TextStyle(fontWeight: FontWeight.bold, color: Colors.white, fontSize: 13), overflow: TextOverflow.ellipsis),
              ),
              IconButton(
                icon: const Icon(Icons.logout, size: 16, color: Colors.grey),
                tooltip: 'Change profile',
                onPressed: () => Navigator.of(context).pushReplacement(
                  MaterialPageRoute(builder: (_) => ProfileSelectionScreen(controller: controller)),
                ),
              ),
            ],
          ),
        ),
        if (controller.tablesLoading)
          Expanded(
            child: Center(
              child: Column(
                mainAxisSize: MainAxisSize.min,
                children: [
                  const SizedBox(width: 24, height: 24, child: CircularProgressIndicator(strokeWidth: 2)),
                  const SizedBox(height: 12),
                  const Text('Connecting to AWS…', style: TextStyle(fontSize: 12, color: Colors.grey)),
                  const SizedBox(height: 4),
                  Text(controller.profile, style: const TextStyle(fontSize: 11, color: Colors.grey)),
                ],
              ),
            ),
          )
        else if (controller.tablesError != null)
          Expanded(
            child: Center(
              child: Padding(
                padding: const EdgeInsets.all(8),
                child: Column(
                  mainAxisSize: MainAxisSize.min,
                  crossAxisAlignment: CrossAxisAlignment.center,
                  children: [
                    Text(
                      controller.tablesError!,
                      style: const TextStyle(color: Colors.redAccent, fontSize: 11),
                      textAlign: TextAlign.center,
                    ),
                    const SizedBox(height: 12),
                    TextButton(onPressed: () => controller.loadTables(), child: const Text('Retry')),
                    const SizedBox(height: 4),
                    TextButton(
                      onPressed: () {
                        Navigator.of(context).pushReplacement(
                          MaterialPageRoute(
                            builder: (_) => ProfileSelectionScreen(controller: controller),
                          ),
                        );
                      },
                      child: const Text('Back to profiles', style: TextStyle(fontSize: 11, color: Colors.grey)),
                    ),
                  ],
                ),
              ),
            ),
          )
        else
          Expanded(
            child: ListView.builder(
              itemCount: controller.tables.length,
              itemBuilder: (context, index) {
                final table = controller.tables[index];
                return ListTile(
                  dense: true,
                  leading: const Icon(Icons.table_chart_outlined, size: 16),
                  title: Text(table, overflow: TextOverflow.ellipsis, style: const TextStyle(fontSize: 13)),
                  onTap: () => controller.openTable(table),
                );
              },
            ),
          ),
      ],
    );
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// MAIN WORKSPACE PANE
// ═══════════════════════════════════════════════════════════════════════════

class _MainWorkspacePane extends StatelessWidget {
  final WorkspaceController controller;
  const _MainWorkspacePane({required this.controller});

  @override
  Widget build(BuildContext context) {
    final c = controller;
    return ListenableBuilder(
      listenable: c,
      builder: (context, _) {
        if (c.activeTable == null) {
          return const Center(child: Text('Nenhuma tabela selecionada.', style: TextStyle(color: Colors.grey)));
        }
        return Column(
          children: [
            _TabBarArea(controller: c),
            const Divider(height: 1, thickness: 1),
            Expanded(child: _TableDashboard(controller: c)),
          ],
        );
      },
    );
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// TAB BAR
// ═══════════════════════════════════════════════════════════════════════════

class _TabBarArea extends StatelessWidget {
  final WorkspaceController controller;
  const _TabBarArea({required this.controller});

  @override
  Widget build(BuildContext context) {
    return SizedBox(
      height: 40,
      child: ListView.builder(
        scrollDirection: Axis.horizontal,
        itemCount: controller.openTables.length,
        itemBuilder: (context, index) {
          final table = controller.openTables[index];
          final isActive = table == controller.activeTable;
          return InkWell(
            onTap: () => controller.selectTable(table),
            child: Container(
              padding: const EdgeInsets.symmetric(horizontal: 16),
              decoration: BoxDecoration(
                color: isActive ? Colors.grey.shade800 : Colors.transparent,
                border: Border(bottom: BorderSide(color: isActive ? Colors.blueAccent : Colors.transparent, width: 2)),
              ),
              child: Row(
                children: [
                  Text(table, style: TextStyle(fontWeight: isActive ? FontWeight.bold : FontWeight.normal, fontSize: 13)),
                  const SizedBox(width: 8),
                  GestureDetector(onTap: () => controller.closeTable(table), child: const Icon(Icons.close, size: 14)),
                ],
              ),
            ),
          );
        },
      ),
    );
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// TABLE DASHBOARD
// ═══════════════════════════════════════════════════════════════════════════

class _TableDashboard extends StatelessWidget {
  final WorkspaceController controller;
  const _TableDashboard({required this.controller});

  @override
  Widget build(BuildContext context) {
    final c = controller;
    return Column(
      children: [
        Container(
          height: 80, padding: const EdgeInsets.all(12), color: Colors.grey.shade900,
          child: Row(
            children: [
              Expanded(
                child: Row(
                  children: [
                    IconButton(icon: const Icon(Icons.refresh, size: 18), tooltip: 'Refresh items', onPressed: () => c.refreshItems()),
                    if (c.hasMorePages)
                      TextButton.icon(
                        onPressed: c.itemsLoading ? null : () => c.loadNextPage(),
                        icon: const Icon(Icons.more_horiz, size: 18),
                        label: const Text('Load more', style: TextStyle(fontSize: 12)),
                      ),
                  ],
                ),
              ),
              TextButton.icon(
                onPressed: () => c.toggleItemDetails(),
                icon: const Icon(Icons.data_object, size: 16),
                label: Text(c.showItemDetails ? 'Hide JSON' : 'Show JSON', style: const TextStyle(fontSize: 12)),
              ),
            ],
          ),
        ),
        Expanded(
          child: Row(
            children: [
              Expanded(child: _buildItemsList(c)),
              if (c.showItemDetails) ...[
                _ResizableDivider(onDrag: (details) => c.resizeDetails(details.delta.dx)),
                SizedBox(width: c.detailsWidth, child: _ItemDetailsPanel(controller: c)),
              ],
            ],
          ),
        ),
      ],
    );
  }

  Widget _buildItemsList(WorkspaceController c) {
    if (c.itemsLoading && c.currentItems.isEmpty) return const Center(child: CircularProgressIndicator());
    if (c.itemsError != null && c.currentItems.isEmpty) return Center(child: Text(c.itemsError!, style: const TextStyle(color: Colors.redAccent)));
    return ListView.separated(
      itemCount: c.currentItems.length,
      separatorBuilder: (_, _) => const Divider(height: 1),
      itemBuilder: (context, index) {
        final item = c.currentItems[index];
        final isSelected = item == c.activeItem;
        return ListTile(
          selected: isSelected,
          selectedTileColor: Colors.blue.withValues(alpha: 0.2),
          title: Text(item.id, style: const TextStyle(fontFamily: 'monospace', fontSize: 13)),
          onTap: () => c.selectItem(index),
        );
      },
    );
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// ITEM DETAILS PANEL
// ═══════════════════════════════════════════════════════════════════════════

class _ItemDetailsPanel extends StatefulWidget {
  final WorkspaceController controller;
  const _ItemDetailsPanel({required this.controller});

  @override
  State<_ItemDetailsPanel> createState() => _ItemDetailsPanelState();
}

class _ItemDetailsPanelState extends State<_ItemDetailsPanel> {
  late final TextEditingController _textController;
  DynamoItem? _trackedItem;
  bool _saving = false;
  bool _deleting = false;

  @override
  void initState() {
    super.initState();
    _textController = TextEditingController();
  }

  @override
  void didChangeDependencies() {
    super.didChangeDependencies();
    final active = widget.controller.activeItem;
    if (active != _trackedItem) {
      _trackedItem = active;
      _textController.text = active.jsonContent;
    }
  }

  @override
  void dispose() {
    _textController.dispose();
    super.dispose();
  }

  Future<void> _save() async {
    setState(() => _saving = true);
    try {
      await widget.controller.saveItem(_textController.text);
    } catch (e) {
      if (mounted) ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text('Save failed: $e')));
    }
    setState(() => _saving = false);
  }

  Future<void> _delete() async {
    setState(() => _deleting = true);
    try {
      await widget.controller.deleteItem();
    } catch (e) {
      if (mounted) ScaffoldMessenger.of(context).showSnackBar(SnackBar(content: Text('Delete failed: $e')));
    }
    setState(() => _deleting = false);
  }

  @override
  Widget build(BuildContext context) {
    final active = widget.controller.activeItem;
    if (active.isEmpty) {
      return Container(
        color: Colors.grey.shade900,
        child: const Center(child: Text('Nenhum item selecionado', style: TextStyle(color: Colors.grey))),
      );
    }
    return Container(
      color: Colors.grey.shade900, padding: const EdgeInsets.all(16),
      child: Column(
        crossAxisAlignment: CrossAxisAlignment.start,
        children: [
          Row(
            mainAxisAlignment: MainAxisAlignment.spaceBetween,
            children: [
              const Text('Editar Item', style: TextStyle(fontWeight: FontWeight.bold, fontSize: 13)),
              Row(
                children: [
                  IconButton(
                    icon: _deleting
                        ? const SizedBox(width: 18, height: 18, child: CircularProgressIndicator(strokeWidth: 2))
                        : const Icon(Icons.delete, color: Colors.redAccent, size: 20),
                    tooltip: 'Remover Item', onPressed: _deleting ? null : _delete,
                  ),
                  IconButton(
                    icon: _saving
                        ? const SizedBox(width: 18, height: 18, child: CircularProgressIndicator(strokeWidth: 2))
                        : const Icon(Icons.save, color: Colors.greenAccent, size: 20),
                    tooltip: 'Salvar JSON', onPressed: _saving ? null : _save,
                  ),
                ],
              ),
            ],
          ),
          const SizedBox(height: 12),
          Expanded(
            child: TextField(
              controller: _textController,
              maxLines: null, expands: true,
              style: const TextStyle(fontFamily: 'monospace', color: Colors.greenAccent, fontSize: 12),
              decoration: const InputDecoration(border: InputBorder.none, filled: true, fillColor: Colors.black26),
            ),
          ),
        ],
      ),
    );
  }
}

// ═══════════════════════════════════════════════════════════════════════════
// RESIZABLE DIVIDER
// ═══════════════════════════════════════════════════════════════════════════

class _ResizableDivider extends StatelessWidget {
  final void Function(DragUpdateDetails) onDrag;
  const _ResizableDivider({required this.onDrag});

  @override
  Widget build(BuildContext context) {
    return MouseRegion(
      cursor: SystemMouseCursors.resizeColumn,
      child: GestureDetector(
        behavior: HitTestBehavior.translucent,
        onPanUpdate: onDrag,
        child: Container(
          width: 6, color: Colors.transparent,
          child: Center(child: Container(width: 1, color: Colors.grey.shade800)),
        ),
      ),
    );
  }
}
