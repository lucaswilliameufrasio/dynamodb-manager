class DynamoItem {
  final String id;
  final String jsonContent;
  final bool isEmpty;

  const DynamoItem({required this.id, required this.jsonContent, this.isEmpty = false});

  factory DynamoItem.empty() => const DynamoItem(id: '', jsonContent: '', isEmpty: true);

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is DynamoItem && runtimeType == other.runtimeType && id == other.id;

  @override
  int get hashCode => id.hashCode;
}
