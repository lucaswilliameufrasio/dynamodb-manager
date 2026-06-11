class DynamoTable {
  final String name;
  final bool isEmpty;

  const DynamoTable({required this.name, this.isEmpty = false});

  factory DynamoTable.empty() => const DynamoTable(name: '', isEmpty: true);

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other is DynamoTable && runtimeType == other.runtimeType && name == other.name;

  @override
  int get hashCode => name.hashCode;
}
