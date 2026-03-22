import { Metadata } from 'next';
import VectorsView from './vectors-view';

export const metadata: Metadata = {
  title: 'Vector Space — BDP',
  description: 'Explore all bioinformatics datasets in semantic embedding space',
};

export default function VectorsPage() {
  return <VectorsView />;
}
