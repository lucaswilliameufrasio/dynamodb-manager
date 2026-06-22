import 'dart:convert';
import 'package:flutter/foundation.dart';
import '../models/dynamo_item.dart';
import '../rust/api/dynamodb.dart' as dynamodb;

class WorkspaceController extends ChangeNotifier {
  // ─── Profile state ─────────────────────────────────────────────────────
  String _profile = '';
  String? _regionOverride;
  String? _endpointOverride;

  String get profile => _profile;
  String? get regionOverride => _regionOverride;
  String? get endpointOverride => _endpointOverride;

  bool get hasProfile => _profile.isNotEmpty;

  void setProfile(String name, {String? region, String? endpoint}) {
    _profile = name;
    _regionOverride = region;
    _endpointOverride = endpoint;
    _tables = [];
    _tablesLoading = false;
    _tablesError = null;
    _openTables.clear();
    _activeTable = null;
    _currentItems = [];
    _itemsLoading = false;
    _itemsError = null;
    _lastEvaluatedKeyJson = null;
    _activeItemIndex = null;
    _showItemDetails = false;
    notifyListeners();
  }

  // ─── Tables state ──────────────────────────────────────────────────────
  List<String> _tables = [];
  bool _tablesLoading = false;
  String? _tablesError;

  List<String> get tables => _tables;
  bool get tablesLoading => _tablesLoading;
  String? get tablesError => _tablesError;

  Future<void> loadTables() async {
    _tablesLoading = true;
    _tablesError = null;
    notifyListeners();

    try {
      _tables = await dynamodb.listTables(
        profile: _profile,
        regionOverride: _regionOverride,
        endpointOverride: _endpointOverride,
      );
      _tablesError = null;
    } catch (e) {
      _tablesError = e.toString();
      _tables = [];
    }

    _tablesLoading = false;
    notifyListeners();
  }

  // ─── Open tabs state ───────────────────────────────────────────────────
  final List<String> _openTables = [];
  String? _activeTable;

  List<String> get openTables => List.unmodifiable(_openTables);
  String? get activeTable => _activeTable;

  void openTable(String tableName) {
    if (!_openTables.contains(tableName)) {
      _openTables.add(tableName);
    }
    _activeTable = tableName;
    _currentItems = [];
    _lastEvaluatedKeyJson = null;
    _activeItemIndex = null;
    _showItemDetails = false;
    _itemsLoading = false;
    _itemsError = null;
    notifyListeners();
    _loadItemsForTable();
  }

  void closeTable(String tableName) {
    _openTables.remove(tableName);
    if (_openTables.isEmpty) {
      _activeTable = null;
      _currentItems = [];
      _showItemDetails = false;
      _activeItemIndex = null;
    } else {
      _activeTable = _openTables.last;
      _currentItems = [];
      _lastEvaluatedKeyJson = null;
      _activeItemIndex = null;
      _showItemDetails = false;
      notifyListeners();
      _loadItemsForTable();
      return;
    }
    notifyListeners();
  }

  void selectTable(String tableName) {
    if (_activeTable != tableName) {
      _activeTable = tableName;
      _currentItems = [];
      _lastEvaluatedKeyJson = null;
      _activeItemIndex = null;
      _showItemDetails = false;
      notifyListeners();
      _loadItemsForTable();
    }
  }

  // ─── Items state ───────────────────────────────────────────────────────
  List<DynamoItem> _currentItems = [];
  bool _showItemDetails = false;
  bool _itemsLoading = false;
  String? _itemsError;
  String? _lastEvaluatedKeyJson;
  int? _activeItemIndex;
  int? _prevActiveItemIndex;

  List<DynamoItem> get currentItems => List.unmodifiable(_currentItems);
  bool get itemsLoading => _itemsLoading;
  String? get itemsError => _itemsError;
  bool get hasMorePages => _lastEvaluatedKeyJson != null;

  DynamoItem get activeItem {
    if (_activeItemIndex == null || _activeItemIndex! >= _currentItems.length) {
      return DynamoItem.empty();
    }
    return _currentItems[_activeItemIndex!];
  }

  bool get showItemDetails => _showItemDetails;

  Future<void> _loadItemsForTable({bool loadMore = false}) async {
    if (_activeTable == null) return;

    if (!loadMore) {
      _currentItems = [];
      _lastEvaluatedKeyJson = null;
    }

    _itemsLoading = true;
    _itemsError = null;
    notifyListeners();

    try {
      final result = await dynamodb.scanItems(
        profile: _profile,
        regionOverride: _regionOverride,
        endpointOverride: _endpointOverride,
        tableName: _activeTable!,
        limit: 50,
        exclusiveStartKeyJson: loadMore ? _lastEvaluatedKeyJson : null,
      );

      for (final jsonStr in result.itemsJson) {
        final data = jsonDecode(jsonStr);
        final id = _extractItemLabel(data);
        const encoder = JsonEncoder.withIndent('  ');
        _currentItems.add(
          DynamoItem(id: id, jsonContent: encoder.convert(data)),
        );
      }

      _lastEvaluatedKeyJson = result.lastEvaluatedKeyJson;
      _itemsError = null;
    } catch (e) {
      _itemsError = e.toString();
    }

    _itemsLoading = false;
    notifyListeners();
  }

  Future<void> loadNextPage() async {
    await _loadItemsForTable(loadMore: true);
  }

  Future<void> refreshItems() async {
    _lastEvaluatedKeyJson = null;
    await _loadItemsForTable();
  }

  String _extractItemLabel(dynamic data) {
    if (data is Map) {
      final keys = data.keys.toList();
      if (keys.isNotEmpty) {
        final firstVal = data[keys.first];
        return '${keys.first}: $firstVal';
      }
    }
    return '(empty item)';
  }

  // ─── Item selection ────────────────────────────────────────────────────
  void selectItem(int index) {
    if (index < 0 || index >= _currentItems.length) return;
    _prevActiveItemIndex = _activeItemIndex;
    _activeItemIndex = index;
    _showItemDetails = true;
    notifyListeners();
  }

  void revertSelection() {
    if (_prevActiveItemIndex != null) {
      _activeItemIndex = _prevActiveItemIndex;
      _prevActiveItemIndex = null;
      notifyListeners();
    }
  }

  void toggleItemDetails() {
    _showItemDetails = !_showItemDetails;
    notifyListeners();
  }

  Future<void> saveItem(String newJson) async {
    if (_activeTable == null || _activeItemIndex == null) return;

    try {
      await dynamodb.putItem(
        profile: _profile,
        regionOverride: _regionOverride,
        endpointOverride: _endpointOverride,
        tableName: _activeTable!,
        itemJson: newJson,
      );

      final updated = DynamoItem(
        id: _currentItems[_activeItemIndex!].id,
        jsonContent: newJson,
      );
      _currentItems[_activeItemIndex!] = updated;
      notifyListeners();
    } catch (e) {
      rethrow;
    }
  }

  Future<void> deleteItem() async {
    if (_activeTable == null || _activeItemIndex == null) return;

    final raw = jsonDecode(_currentItems[_activeItemIndex!].jsonContent);
    if (raw is! Map) return;

    // Build key from known key schema — fallback: send all top-level attributes as key
    final keyJson = jsonEncode(raw);

    try {
      await dynamodb.deleteItem(
        profile: _profile,
        regionOverride: _regionOverride,
        endpointOverride: _endpointOverride,
        tableName: _activeTable!,
        keyJson: keyJson,
      );

      _currentItems.removeAt(_activeItemIndex!);
      _activeItemIndex = null;
      _showItemDetails = false;
      notifyListeners();
    } catch (e) {
      rethrow;
    }
  }

  // ─── Layout resizing ───────────────────────────────────────────────────
  double _sidebarWidth = 250.0;
  double _detailsWidth = 350.0;

  double get sidebarWidth => _sidebarWidth;
  double get detailsWidth => _detailsWidth;

  void resizeSidebar(double delta) {
    _sidebarWidth += delta;
    if (_sidebarWidth < 150) _sidebarWidth = 150;
    if (_sidebarWidth > 600) _sidebarWidth = 600;
    notifyListeners();
  }

  void resizeDetails(double delta) {
    _detailsWidth -= delta;
    if (_detailsWidth < 200) _detailsWidth = 200;
    if (_detailsWidth > 800) _detailsWidth = 800;
    notifyListeners();
  }
}
