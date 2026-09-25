import 'package:flutter/material.dart';

import '../models/dynamo_item.dart';

class DynamoItemsList extends StatelessWidget {
  final List<DynamoItem> Function() itemsProvider;
  final bool loading;
  final String? error;
  final DynamoItem? Function() selectedItemProvider;
  final ValueChanged<int> onSelect;

  const DynamoItemsList({
    required this.itemsProvider,
    required this.onSelect,
    required this.selectedItemProvider,
    this.loading = false,
    this.error,
    super.key,
  });

  @override
  Widget build(BuildContext context) {
    if (loading && itemsProvider().isEmpty) {
      return const Center(child: CircularProgressIndicator());
    }
    if (error != null && itemsProvider().isEmpty) {
      return Center(
        child: Text(error!, style: const TextStyle(color: Colors.redAccent)),
      );
    }
    return ListView.separated(
      itemCount: itemsProvider().length,
      separatorBuilder: (_, _) => const Divider(height: 1),
      itemBuilder: (context, index) {
        final item = itemsProvider()[index];
        return ListTile(
          selected: item == selectedItemProvider(),
          selectedTileColor: Colors.blue.withValues(alpha: 0.2),
          title: Text(
            item.id,
            style: const TextStyle(fontFamily: 'monospace', fontSize: 13),
          ),
          onTap: () => onSelect(index),
        );
      },
    );
  }
}
